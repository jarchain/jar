import Genesis.Test.GenesisJson

open Genesis.Test.GenesisJson

def genesisJsonMain (args : List String) : IO UInt32 := do
  let (bless, dir) := match args with
    | ["--bless", d] => (true, d)
    | ["--bless"] => (true, "tests/vectors/genesis")
    | [d] => (false, d)
    | _ => (false, "tests/vectors/genesis")
  IO.println s!"Running Genesis tests from: {dir}{if bless then " (bless mode)" else ""}"
  runJsonTestDir dir bless
