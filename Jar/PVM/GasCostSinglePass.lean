import Jar.PVM.GasCost

/-!
# PVM Gas Cost — Single-Pass Model

O(n) single-pass gas cost computation. Tracks per-register completion cycles
instead of a full ROB simulation. Implicitly models register renaming
(only the last writer per register matters).

Cost = `max(maxDone - 3, 1)`.

See `docs/gas-metering-design.md` for detailed comparison with the full pipeline model.
-/

namespace Jar.PVM

/-- Single-pass simulation state. -/
structure GasSimStateSP where
  ι         : Option Nat    -- current instruction PC (none = done)
  cycle     : Nat           -- current decode cycle
  decodeUsed : Nat          -- decode slots consumed this cycle
  regDone   : Array Nat     -- cycle when each register's value is ready (13 entries)
  maxDone   : Nat           -- max completion cycle across all instructions

/-- Single-pass gas simulation: process one instruction at a time. -/
partial def gasSimSinglePass (code bitmask : ByteArray) (s : GasSimStateSP) : GasSimStateSP :=
  match s.ι with
  | none => s
  | some pc =>
    let cost := instructionCost code bitmask pc
    -- Advance cycle if decode slots would overflow
    let (cycle, decodeUsed) :=
      if s.decodeUsed + cost.decodeSlots > 4
      then (s.cycle + 1, cost.decodeSlots)
      else (s.cycle, s.decodeUsed + cost.decodeSlots)
    let nextι := if cost.isTerminator then none
                 else nextInstrPC bitmask pc
    if cost.isMoveReg then
      -- move_reg: 0-cycle frontend operation, propagate regDone from src to dst
      let regDone := if cost.destRegs.size > 0 && cost.srcRegs.size > 0 then
        let srcReg := cost.srcRegs[0]!
        let srcDone := if srcReg < s.regDone.size then s.regDone[srcReg]! else 0
        cost.destRegs.foldl (fun rd r =>
          if r < rd.size then rd.set! r srcDone else rd) s.regDone
      else s.regDone
      gasSimSinglePass code bitmask { s with ι := nextι, cycle := cycle, decodeUsed := decodeUsed, regDone := regDone }
    else
      -- Start cycle = max(decode_cycle, max(regDone[src_regs]))
      let start := cost.srcRegs.foldl (fun acc r =>
        if r < s.regDone.size then max acc s.regDone[r]! else acc) cycle
      let done := start + cost.cycles
      -- Update regDone for destination registers
      let regDone := cost.destRegs.foldl (fun rd r =>
        if r < rd.size then rd.set! r done else rd) s.regDone
      let maxDone := max s.maxDone done
      gasSimSinglePass code bitmask { ι := nextι, cycle := cycle, decodeUsed := decodeUsed, regDone := regDone, maxDone := maxDone }

/-- Compute gas cost for a basic block using the single-pass model.
    Returns `max(maxDone - 3, 1)`. -/
def gasCostForBlockSinglePass (code bitmask : ByteArray) (startPC : Nat) : Nat :=
  let initState : GasSimStateSP := {
    ι := some startPC
    cycle := 0
    decodeUsed := 0
    regDone := #[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    maxDone := 0
  }
  let finalState := gasSimSinglePass code bitmask initState
  if finalState.maxDone > 3 then finalState.maxDone - 3 else 1

end Jar.PVM
