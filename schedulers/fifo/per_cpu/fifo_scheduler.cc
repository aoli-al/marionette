// Copyright 2021 Google LLC
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file or at
// https://developers.google.com/open-source/licenses/bsd

#include "schedulers/fifo/per_cpu/fifo_scheduler.h"

#include <memory>
#include <unistd.h>

namespace ghost {

FifoScheduler::FifoScheduler(Enclave* enclave, CpuList cpulist,
                             std::shared_ptr<TaskAllocator<FifoTask>> allocator)
    : BasicDispatchScheduler(enclave, std::move(cpulist),
                             std::move(allocator)) {
  for (const Cpu& cpu : cpus()) {
    // TODO: extend Cpu to get numa node.
    int node = 0;
    CpuState* cs = cpu_state(cpu);
    cs->channel = enclave->MakeChannel(GHOST_MAX_QUEUE_ELEMS, node,
                                       MachineTopology()->ToCpuList({cpu}));
    // This channel pointer is valid for the lifetime of FifoScheduler
    if (!default_channel_) {
      default_channel_ = cs->channel.get();
    }
  }
}

void FifoScheduler::DumpAllTasks() {
  fprintf(stderr, "task        state   cpu\n");
  allocator()->ForEachTask([](Gtid gtid, const FifoTask* task) {
    absl::FPrintF(stderr, "%-12s%-8d\n", gtid.describe(), task->cpu);
    return true;
  });
}

void FifoScheduler::DumpState(const Cpu& cpu, int flags) {
}

void FifoScheduler::EnclaveReady() {
  for (const Cpu& cpu : cpus()) {
    CpuState* cs = cpu_state(cpu);
    Agent* agent = enclave()->GetAgent(cpu);

    // AssociateTask may fail if agent barrier is stale.
    while (!cs->channel->AssociateTask(agent->gtid(), agent->barrier(),
                                       /*status=*/nullptr)) {
      CHECK_EQ(errno, ESTALE);
    }
  }
}

// Implicitly thread-safe because it is only called from one agent associated
// with the default queue.
Cpu FifoScheduler::AssignCpu(FifoTask* task) {
  static auto begin = cpus().begin();
  static auto end = cpus().end();
  static auto next = end;

  if (next == end) {
    next = begin;
  }
  return next++;
}

void FifoScheduler::Migrate(FifoTask* task, Cpu cpu, BarrierToken seqnum) {
  CHECK_EQ(task->cpu, -1);

  CpuState* cs = cpu_state(cpu);
  const Channel* channel = cs->channel.get();
  CHECK(channel->AssociateTask(task->gtid, seqnum, /*status=*/nullptr));

  task->cpu = cpu.id();

  // Make task visible in the new runqueue *after* changing the association
  // (otherwise the task can get oncpu while producing into the old queue).
  cs->run_queue.Push(task);

  // Get the agent's attention so it notices the new task.
  enclave()->GetAgent(cpu)->Ping();
}

void FifoScheduler::TaskNew(FifoTask* task, const Message& msg) {
  const ghost_msg_payload_task_new* payload =
      static_cast<const ghost_msg_payload_task_new*>(msg.payload());

  task->seqnum = msg.seqnum();
  GHOST_DPRINT(1, stderr, "New task found: %s", task->gtid.describe());
  if (payload->runnable) {
    should_reschedule_ = true;
    Cpu cpu = AssignCpu(task);
    Migrate(task, cpu, msg.seqnum());
  } else {
    // Wait until task becomes runnable to avoid race between migration
    // and MSG_TASK_WAKEUP showing up on the default channel.
  }
}

void FifoScheduler::TaskRunnable(FifoTask* task, const Message& msg) {
  should_reschedule_ = true;
  GHOST_DPRINT(1, stderr, "Task runnable: %s", task->gtid.describe());
  if (task->cpu < 0) {
    // There cannot be any more messages pending for this task after a
    // MSG_TASK_WAKEUP (until the agent puts it oncpu) so it's safe to
    // migrate.
    Cpu cpu = AssignCpu(task);
    Migrate(task, cpu, msg.seqnum());
  } else {
    CpuState* cs = cpu_state_of(task);
    cs->run_queue.Push(task);
  }
}

void FifoScheduler::TaskDeparted(FifoTask* task, const Message& msg) {
  const ghost_msg_payload_task_departed* payload =
      static_cast<const ghost_msg_payload_task_departed*>(msg.payload());
  GHOST_DPRINT(1, stderr, "Task departed: %s", task->gtid.describe());
  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Erase(task);
  if (payload->from_switchto) {
    Cpu cpu = topology()->cpu(payload->cpu);
    enclave()->GetAgent(cpu)->Ping();
  }
  allocator()->FreeTask(task);
}

void FifoScheduler::TaskDead(FifoTask* task, const Message& msg) {
  GHOST_DPRINT(1, stderr, "Task dead: %s", task->gtid.describe());
  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Erase(task);
  allocator()->FreeTask(task);
}

void FifoScheduler::TaskYield(FifoTask* task, const Message& msg) {
  GHOST_DPRINT(1, stderr, "Task yield: %s", task->gtid.describe());
  should_reschedule_ = true;
  const ghost_msg_payload_task_yield* payload =
      static_cast<const ghost_msg_payload_task_yield*>(msg.payload());

  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Push(task);

  if (payload->from_switchto) {
    Cpu cpu = topology()->cpu(payload->cpu);
    enclave()->GetAgent(cpu)->Ping();
  }
}

void FifoScheduler::TaskBlocked(FifoTask* task, const Message& msg) {
  GHOST_DPRINT(1, stderr, "Task blocked: %s", task->gtid.describe());
  should_reschedule_ = true;
  const ghost_msg_payload_task_blocked* payload =
      static_cast<const ghost_msg_payload_task_blocked*>(msg.payload());

  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Push(task, FifoTaskState::kBlocked);

  if (payload->from_switchto) {
    Cpu cpu = topology()->cpu(payload->cpu);
    enclave()->GetAgent(cpu)->Ping();
  }
}

void FifoScheduler::TaskPreempted(FifoTask* task, const Message& msg) {
  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Push(task);
  const ghost_msg_payload_task_preempt* payload =
      static_cast<const ghost_msg_payload_task_preempt*>(msg.payload());
  if (payload->from_switchto) {
    Cpu cpu = topology()->cpu(payload->cpu);
    enclave()->GetAgent(cpu)->Ping();
  }
}

void FifoScheduler::TaskSwitchto(FifoTask* task, const Message& msg) {
  GHOST_DPRINT(1, stderr, "Task switchto: %s", task->gtid.describe());
  should_reschedule_ = true;
  CpuState* cs = cpu_state_of(task);
  cs->run_queue.Push(task, FifoTaskState::kBlocked);
}


void FifoScheduler::FifoSchedule(const Cpu& cpu, BarrierToken agent_barrier,
                                 bool prio_boost) {
  CpuState* cs = cpu_state(cpu);
  FifoTask* next = nullptr;
  if (!prio_boost) {
    next = cs->run_queue.GetCurrentTask();
    if (!next || should_reschedule_) {
      next = cs->run_queue.GetRandom();
      if (next) {
        GHOST_DPRINT(1, stderr, "Scheduled thread: %s", next->gtid.describe());
      }
      should_reschedule_ = false;
    }
  }

  RunRequest* req = enclave()->GetRunRequest(cpu);
  if (next) {
    cs->run_queue.SetCurrentTaskId(next->gtid);
    req->Open({
        .target = next->gtid,
        .target_barrier = next->seqnum,
        .agent_barrier = agent_barrier,
        .commit_flags = COMMIT_AT_TXN_COMMIT,
    });

    if (req->Commit()) {
      // Txn commit succeeded and 'next' is oncpu.
    } else {
      GHOST_DPRINT(1, stderr, "FifoSchedule: commit failed (state=%d)",
                   req->state());
      stale_commit_ = true;
    }
  } else {
    int flags = 0;
    if (prio_boost) {
      flags = RTLA_ON_IDLE;
    }
    req->LocalYield(agent_barrier, flags);
  }
}

void FifoScheduler::Schedule(const Cpu& cpu, const StatusWord& agent_sw) {
  BarrierToken agent_barrier = agent_sw.barrier();
  CpuState* cs = cpu_state(cpu);

  GHOST_DPRINT(3, stderr, "Schedule: agent_barrier[%d] = %d\n", cpu.id(),
               agent_barrier);

  Message msg;
  while (!(msg = Peek(cs->channel.get())).empty()) {
    DispatchMessage(msg);
    Consume(cs->channel.get(), msg);
  }

  FifoSchedule(cpu, agent_barrier, agent_sw.boosted_priority());
}

void TaskVector::Push(FifoTask* task, FifoTaskState state) {
  CHECK_GE(task->cpu, 0);
  absl::MutexLock lock(&mu_);
  tasks_.insert(task->gtid.id());
  task_map_[task->gtid.id()] = task;
  task_status_[task->gtid.id()] = state;
}

FifoTask* TaskVector::GetRandom() {
  absl::MutexLock lock(&mu_);
  std::vector<int64_t> runnable;
  for (auto gtid: tasks_) {
    if (task_status_[gtid] == FifoTaskState::kRunnable) {
      runnable.push_back(gtid);
    }
  }
  if (runnable.empty()) {
    return nullptr;
  }
  GHOST_DPRINT(1, stderr, "Num available threads: %d", runnable.size());

  size_t index = std::rand() % runnable.size();
  FifoTask* task = task_map_[runnable[index]];
  return task;
}

void TaskVector::Erase(FifoTask* task) {
  absl::MutexLock lock(&mu_);
  task_map_.erase(task->gtid.id());
  tasks_.erase(task->gtid.id());
  task_status_.erase(task->gtid.id());
  if (task->gtid == current_) {
    current_ = Gtid();
  }
}

void TaskVector::SetTaskState(Gtid task_id, FifoTaskState state) {
  absl::MutexLock lock(&mu_);
  task_status_[task_id.id()] = state;
}
FifoTaskState TaskVector::GetTaskState(Gtid task_id) {
  absl::MutexLock lock(&mu_);
  return task_status_[task_id.id()];
}


std::unique_ptr<FifoScheduler> MultiThreadedFifoScheduler(Enclave* enclave,
                                                          CpuList cpulist) {
  auto allocator = std::make_shared<ThreadSafeMallocTaskAllocator<FifoTask>>();
  auto scheduler = std::make_unique<FifoScheduler>(enclave, std::move(cpulist),
                                                   std::move(allocator));
  return scheduler;
}

void FifoAgent::AgentThread() {
  gtid().assign_name("Agent:" + std::to_string(cpu().id()));
  if (verbose() > 1) {
    printf("Agent tid:=%d\n", gtid().tid());
  }
  SignalReady();
  WaitForEnclaveReady();

  PeriodicEdge debug_out(absl::Seconds(1));

  while (!Finished() || !scheduler_->Empty(cpu())) {
    scheduler_->Schedule(cpu(), status_word());

    if (verbose() && debug_out.Edge()) {
      static const int flags = verbose() > 1 ? Scheduler::kDumpStateEmptyRQ : 0;
      if (scheduler_->debug_runqueue_) {
        scheduler_->debug_runqueue_ = false;
        scheduler_->DumpState(cpu(), Scheduler::kDumpAllTasks);
      } else {
        scheduler_->DumpState(cpu(), flags);
      }
    }
  }
}

std::ostream& operator<<(std::ostream& os, const FifoTaskState& state) {
  switch (state) {
    case FifoTaskState::kBlocked:
      return os << "kBlocked";
    case FifoTaskState::kRunnable:
      return os << "kRunnable";
  }
  return os;
}

}  //  namespace ghost
