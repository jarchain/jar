import Jar.Test.BlockTest
import Jar.Variant

open Jar Jar.Test.BlockTest

def testVariants : Array JamConfig := #[JamVariant.gp072_tiny.toJamConfig]

def traceNames : Array String := #["safrole"]

def main (args : List String) : IO UInt32 := do
  let mut exitCode : UInt32 := 0
  match args with
  | [d] =>
    -- Single directory mode
    for v in testVariants do
      letI := v
      IO.println s!"Running block tests ({v.name}) from: {d}"
      let code ← runBlockTestDir d
      if code != 0 then exitCode := code
  | _ =>
    -- Run all trace directories
    for trace in traceNames do
      let dir := s!"tests/vectors/blocks/{trace}"
      for v in testVariants do
        letI := v
        IO.println s!"Running block tests ({v.name}) from: {dir}"
        let code ← runBlockTestDir dir
        if code != 0 then exitCode := code
  return exitCode
