#include "InjectYieldPass.hpp"
#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/Casting.h"
#include <iostream>
#include <llvm-17/llvm/IR/InstrTypes.h>
#include <llvm-17/llvm/Support/raw_ostream.h>

namespace marionette {

bool InjectYieldPass::shouldInstrument(CallBase &CB) {
  auto Callee = CB.getCalledFunction();
  if (!Callee->hasName()) {
    return false;
  }

  auto Name = Callee->getName();

  if (Name.contains("drop_in_place") && Name.contains("MutexGuard")) {
    return true;
  }
  return false;
}

bool InjectYieldPass::instrumentCallBase(CallBase &CB, BasicBlock &BB) {
  auto &Ctx = CB.getContext();
  FunctionCallee YieldFunc = CB.getModule()->getOrInsertFunction(
      "sched_yield",
      FunctionType::get(IntegerType::getInt32Ty(Ctx), false));

  if (!shouldInstrument(CB)) {
    return false;
  }

  if (isa<CallInst>(CB)) {
    IRBuilder<> Builder(&CB);
    Builder.SetInsertPoint(&BB, ++Builder.GetInsertPoint());
    Builder.CreateCall(YieldFunc);
    return true;
  }
  if (isa<InvokeInst>(CB)) {
    auto &II = cast<InvokeInst>(CB);
    auto DestBB = II.getNormalDest();
    if (DestBB != nullptr) {
      llvm::outs() << *DestBB;
      IRBuilder<> Builder(DestBB);
      Builder.CreateCall(YieldFunc);
      return true;
    } else {
      llvm::outs() << "Empty Dest BB?";
    }
  }
  return false;
}

bool InjectYieldPass::runOnModule(Module &M) {
  auto &Context = M.getContext();

  bool instrumented = false;

  for (auto &F : M) {
    for (auto &BB : F) {
      for (auto &I : BB) {
        if (isa<CallBase>(I)) {
          instrumented |= instrumentCallBase(cast<CallBase>(I), BB);
        }
      }
    }
  }
  return instrumented;
}

PreservedAnalyses InjectYieldPass::run(llvm::Module &M,
                                       llvm::ModuleAnalysisManager &) {
  bool Changed = runOnModule(M);

  return (Changed ? llvm::PreservedAnalyses::none()
                  : llvm::PreservedAnalyses::all());
}

} // namespace marionette

PassPluginLibraryInfo getInjectFuncCallPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "inject-yield", LLVM_VERSION_STRING,
          [](PassBuilder &PB) {
            PB.registerPipelineParsingCallback(
                [](StringRef Name, ModulePassManager &MPM,
                   ArrayRef<PassBuilder::PipelineElement>) {
                  if (Name == "inject-yield") {
                    MPM.addPass(marionette::InjectYieldPass());
                    return true;
                  }
                  return false;
                });
          }};
}

extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return getInjectFuncCallPluginInfo();
}