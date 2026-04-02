import Jar.Test.StfServer
import Jar.Variant

open Jar

def main (args : List String) : IO UInt32 := do
  match args with
  | "--variant" :: v :: rest =>
    -- Each branch calls StfServer.main directly with its own JamConfig instance,
    -- avoiding type-level incompatibility between variants with different EconType.
    match v with
      | "gp072_tiny" => letI := JamVariant.gp072_tiny.toJamConfig; Jar.Test.StfServer.main rest
      | "gp072_full" => letI := JamVariant.gp072_full.toJamConfig; Jar.Test.StfServer.main rest
      | "jar1" => letI := JamVariant.jar1.toJamConfig; Jar.Test.StfServer.main rest
      | _ =>
        IO.eprintln s!"Unknown variant: {v}"
        IO.eprintln "Available: gp072_tiny, gp072_full, jar1"
        return 1
  | _ =>
    IO.eprintln "Usage: jar-stf --variant <variant> [--bless] <sub-transition> <input|dir>"
    IO.eprintln "Variants: gp072_tiny, gp072_full, jar1"
    return 1
