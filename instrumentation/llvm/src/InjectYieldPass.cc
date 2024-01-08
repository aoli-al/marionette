#include "InjectYieldPass.hpp"
#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/InstrTypes.h"
#include "llvm/IR/Instructions.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/Casting.h"
#include "llvm/Support/raw_ostream.h"
#include <iostream>
#include <llvm-17/llvm/IR/DerivedTypes.h>

namespace marionette {

bool InjectYieldPass::shouldInstrument(CallBase &CB) {
  auto Callee = CB.getCalledFunction();
  if (Callee == nullptr || !Callee->hasName()) {
    return false;
  }

  auto Name = Callee->getName();

  if (Name.contains("drop_in_place") && Name.contains("MutexGuard")) {
    return true;
  }
  return false;
}

bool InjectYieldPass::instrumentBB(BasicBlock *BB, FunctionCallee &Callee) {
  if (this->InstrumentedBlocks.contains(BB)) {
    return false;
  }
  this->InstrumentedBlocks.insert(BB);
  IRBuilder<> Builder(BB);
  Builder.SetInsertPoint(BB, BB->begin());
  Builder.CreateCall(Callee);
  return true;
}

bool InjectYieldPass::instrumentCallBase(CallBase &CB, BasicBlock &BB) {
  auto &Ctx = CB.getContext();
  FunctionCallee YieldFunc = CB.getModule()->getOrInsertFunction(
      "sched_yield", FunctionType::get(IntegerType::getInt32Ty(Ctx), false));
  if (!shouldInstrument(CB)) {
    return false;
  }

  // llvm::outs() << "start instrument: " << CB;

  if (isa<CallInst>(CB)) {
    llvm::outs() << "Instrument CI: " << CB;
    IRBuilder<> Builder(&CB);
    Builder.SetInsertPoint(&BB, ++Builder.GetInsertPoint());
    Builder.CreateCall(YieldFunc);
    return false;
  }
  if (isa<InvokeInst>(CB)) {
    llvm::outs() << "Instrument II: " << CB;
    auto &II = cast<InvokeInst>(CB);
    auto DestBB = II.getNormalDest();
    if (DestBB != nullptr) {
      return instrumentBB(DestBB, YieldFunc);
    } else {
      llvm::outs() << "Empty Dest BB?";
    }
  }
  return false;
}

bool InjectYieldPass::runOnModule(Module &M) {
  auto &Context = M.getContext();

  bool Instrumented = false;

  for (auto &F : M) {
    for (auto &BB : F) {
      for (auto &I : BB) {
        if (isa<CallBase>(I)) {
          Instrumented |= instrumentCallBase(cast<CallBase>(I), BB);
        }
      }
    }
  }
  return Instrumented;
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