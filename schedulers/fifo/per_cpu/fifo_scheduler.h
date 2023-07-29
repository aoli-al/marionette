// Copyright 2021 Google LLC
//
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file or at
// https://developers.google.com/open-source/licenses/bsd

#ifndef GHOST_SCHEDULERS_FIFO_FIFO_SCHEDULER_H
#define GHOST_SCHEDULERS_FIFO_FIFO_SCHEDULER_H

#include <deque>
#include <memory>

#include "lib/agent.h"
#include "lib/scheduler.h"

namespace ghost {

enum class FifoTaskState {
  kBlocked,   // not on runqueue.
  kRunnable,  // transitory state:
};

// For CHECK and friends.
std::ostream& operator<<(std::ostream& os, const FifoTaskState& state);

struct FifoTask : public Task<> {
  explicit FifoTask(Gtid fifo_task_gtid, ghost_sw_info sw_info)
      : Task<>(fifo_task_gtid, sw_info) {}
  ~FifoTask() override {}
  int cpu = -1;
};

class TaskVector {
 public:
  TaskVector() = default;
  TaskVector(const TaskVector&) = delete;
  TaskVector& operator=(TaskVector&) = delete;

  FifoTask* GetRandom();
  void Push(FifoTask* task, FifoTaskState state = FifoTaskState::kRunnable);
  void Erase(FifoTask* task);
  void SetTaskState(Gtid task_id, FifoTaskState state);
  FifoTaskState GetTaskState(Gtid task_id);

  size_t Size() const {
    absl::MutexLock lock(&mu_);
    return tasks_.size();
  }

  FifoTask * GetCurrentTask() {
    if (current_.id() == -1) {
      return nullptr;
    }
    return task_map_[current_.id()];
  }

  void SetCurrentTaskId(Gtid id) {
    current_ = id;
  }

  bool Empty() const { return Size() == 0; }

 private:
  mutable absl::Mutex mu_;
  Gtid current_;
  std::unordered_set<int64_t> tasks_ ABSL_GUARDED_BY(mu_);
  std::unordered_map<int64_t, FifoTask*> task_map_ ABSL_GUARDED_BY(mu_);
  std::unordered_map<int64_t, FifoTaskState> task_status_ ABSL_GUARDED_BY(mu_);
};

class FifoScheduler : public BasicDispatchScheduler<FifoTask> {
 public:
  explicit FifoScheduler(Enclave* enclave, CpuList cpulist,
                         std::shared_ptr<TaskAllocator<FifoTask>> allocator);
  ~FifoScheduler() final {}

  void Schedule(const Cpu& cpu, const StatusWord& sw);

  void EnclaveReady() final;
  Channel& GetDefaultChannel() final { return *default_channel_; };

  bool Empty(const Cpu& cpu) {
    CpuState* cs = cpu_state(cpu);
    return cs->run_queue.Empty();
  }

  void DumpState(const Cpu& cpu, int flags) final;
  std::atomic<bool> debug_runqueue_ = false;

  int CountAllTasks() {
    int num_tasks = 0;
    allocator()->ForEachTask([&num_tasks](Gtid gtid, const FifoTask* task) {
      ++num_tasks;
      return true;
    });
    return num_tasks;
  }

  static constexpr int kDebugRunqueue = 1;
  static constexpr int kCountAllTasks = 2;

 protected:
  void TaskNew(FifoTask* task, const Message& msg) final;
  void TaskRunnable(FifoTask* task, const Message& msg) final;
  void TaskDeparted(FifoTask* task, const Message& msg) final;
  void TaskDead(FifoTask* task, const Message& msg) final;
  void TaskYield(FifoTask* task, const Message& msg) final;
  void TaskBlocked(FifoTask* task, const Message& msg) final;
  void TaskPreempted(FifoTask* task, const Message& msg) final;
  void TaskSwitchto(FifoTask* task, const Message& msg) final;

 private:
  void FifoSchedule(const Cpu& cpu, BarrierToken agent_barrier,
                    bool prio_boosted);
  void TaskOffCpu(FifoTask* task, bool blocked, bool from_switchto);
  void TaskOnCpu(FifoTask* task, Cpu cpu);
  void Migrate(FifoTask* task, Cpu cpu, BarrierToken seqnum);
  Cpu AssignCpu(FifoTask* task);
  void DumpAllTasks();

  struct CpuState {
    std::unique_ptr<Channel> channel = nullptr;
    TaskVector run_queue;
  } ABSL_CACHELINE_ALIGNED;

  inline CpuState* cpu_state(const Cpu& cpu) { return &cpu_states_[cpu.id()]; }

  inline CpuState* cpu_state_of(const FifoTask* task) {
    CHECK_GE(task->cpu, 0);
    CHECK_LT(task->cpu, MAX_CPUS);
    return &cpu_states_[task->cpu];
  }

  CpuState cpu_states_[MAX_CPUS];
  Channel* default_channel_ = nullptr;
  bool should_reschedule_ = true;
  bool stale_commit_ = false;
};

std::unique_ptr<FifoScheduler> MultiThreadedFifoScheduler(Enclave* enclave,
                                                          CpuList cpulist);
class FifoAgent : public LocalAgent {
 public:
  FifoAgent(Enclave* enclave, Cpu cpu, FifoScheduler* scheduler)
      : LocalAgent(enclave, cpu), scheduler_(scheduler) {}

  void AgentThread() override;
  Scheduler* AgentScheduler() const override { return scheduler_; }

 private:
  FifoScheduler* scheduler_;
};

template <class EnclaveType>
class FullFifoAgent : public FullAgent<EnclaveType> {
 public:
  explicit FullFifoAgent(AgentConfig config) : FullAgent<EnclaveType>(config) {
    scheduler_ =
        MultiThreadedFifoScheduler(&this->enclave_, *this->enclave_.cpus());
    this->StartAgentTasks();
    this->enclave_.Ready();
  }

  ~FullFifoAgent() override {
    this->TerminateAgentTasks();
  }

  std::unique_ptr<Agent> MakeAgent(const Cpu& cpu) override {
    return std::make_unique<FifoAgent>(&this->enclave_, cpu, scheduler_.get());
  }

  void RpcHandler(int64_t req, const AgentRpcArgs& args,
                  AgentRpcResponse& response) override {
    switch (req) {
      case FifoScheduler::kDebugRunqueue:
        scheduler_->debug_runqueue_ = true;
        response.response_code = 0;
        return;
      case FifoScheduler::kCountAllTasks:
        response.response_code = scheduler_->CountAllTasks();
        return;
      default:
        response.response_code = -1;
        return;
    }
  }

 private:
  std::unique_ptr<FifoScheduler> scheduler_;
};

}  // namespace ghost

#endif  // GHOST_SCHEDULERS_FIFO_FIFO_SCHEDULER_H
