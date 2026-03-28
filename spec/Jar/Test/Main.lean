/-
  jartest — unified test runner for JAR specification.

  Subcommands:
    safrole, history, statistics, authorizations, disputes, preimages,
    assurances, reports, accumulate, property, trie, shuffle, codec,
    block, erasure, crypto, genesis

  Usage: jartest <subcommand> [args...]
  Without subcommand: runs all tests.
-/

import Cli
import Jar.Test.SafroleJsonMain
import Jar.Test.HistoryJsonMain
import Jar.Test.StatisticsJsonMain
import Jar.Test.AuthorizationsJsonMain
import Jar.Test.DisputesJsonMain
import Jar.Test.PreimagesJsonMain
import Jar.Test.AssurancesJsonMain
import Jar.Test.ReportsJsonMain
import Jar.Test.AccumulateJsonMain
import Jar.Test.PropertyMain
import Jar.Test.TrieTestMain
import Jar.Test.ShuffleTestMain
import Jar.Test.CodecTestMain
import Jar.Test.BlockTestMain
import Jar.Test.ErasureTestMain
import Jar.CryptoTest
import Genesis.Test.GenesisJsonMain

open Cli

-- Wrappers: lean4-cli gives us Parsed, but test mains take (args : List String) or no args.
-- Pass remaining args through.
def runSafrole (p : Parsed) : IO UInt32 := safroleJsonMain (p.variableArgsAs! String |>.toList)
def runHistory (p : Parsed) : IO UInt32 := historyJsonMain (p.variableArgsAs! String |>.toList)
def runStatistics (p : Parsed) : IO UInt32 := statisticsJsonMain (p.variableArgsAs! String |>.toList)
def runAuthorizations (p : Parsed) : IO UInt32 := authorizationsJsonMain (p.variableArgsAs! String |>.toList)
def runDisputes (p : Parsed) : IO UInt32 := disputesJsonMain (p.variableArgsAs! String |>.toList)
def runPreimages (p : Parsed) : IO UInt32 := preimagesJsonMain (p.variableArgsAs! String |>.toList)
def runAssurances (p : Parsed) : IO UInt32 := assurancesJsonMain (p.variableArgsAs! String |>.toList)
def runReports (p : Parsed) : IO UInt32 := reportsJsonMain (p.variableArgsAs! String |>.toList)
def runAccumulate (p : Parsed) : IO UInt32 := accumulateJsonMain (p.variableArgsAs! String |>.toList)
def runProperty (p : Parsed) : IO UInt32 := propertyMain
def runTrie (p : Parsed) : IO UInt32 := trieTestMain
def runShuffle (p : Parsed) : IO UInt32 := shuffleTestMain
def runCodec (p : Parsed) : IO UInt32 := codecTestMain (p.variableArgsAs! String |>.toList)
def runBlock (p : Parsed) : IO UInt32 := blockTestMain (p.variableArgsAs! String |>.toList)
def runErasure (p : Parsed) : IO UInt32 := erasureTestMain
def runCrypto (p : Parsed) : IO UInt32 := do cryptoTestMain; return 0
def runGenesis (p : Parsed) : IO UInt32 := genesisJsonMain (p.variableArgsAs! String |>.toList)
-- stf (jarstf) kept as separate exe — it's a bless-mode server, not a test

def safroleCmd := `[Cli| safrole VIA runSafrole; "Run Safrole JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def historyCmd := `[Cli| history VIA runHistory; "Run history JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def statisticsCmd := `[Cli| statistics VIA runStatistics; "Run statistics JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def authorizationsCmd := `[Cli| authorizations VIA runAuthorizations; "Run authorizations JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def disputesCmd := `[Cli| disputes VIA runDisputes; "Run disputes JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def preimagesCmd := `[Cli| preimages VIA runPreimages; "Run preimages JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def assurancesCmd := `[Cli| assurances VIA runAssurances; "Run assurances JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def reportsCmd := `[Cli| reports VIA runReports; "Run reports JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def accumulateCmd := `[Cli| accumulate VIA runAccumulate; "Run accumulate JSON conformance tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def propertyCmd := `[Cli| property VIA runProperty; "Run property-based tests." ]
def trieCmd := `[Cli| trie VIA runTrie; "Run Merkle trie tests." ]
def shuffleCmd := `[Cli| shuffle VIA runShuffle; "Run Safrole shuffle tests." ]
def codecCmd := `[Cli| codec VIA runCodec; "Run codec roundtrip tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def blockCmd := `[Cli| block VIA runBlock; "Run full block tests."
  ARGS: ...dirs : String; "Test vector directories." ]
def erasureCmd := `[Cli| erasure VIA runErasure; "Run Reed-Solomon erasure coding tests." ]
def cryptoCmd := `[Cli| crypto VIA runCrypto; "Run cryptographic primitive tests." ]
def genesisCmd := `[Cli| genesis VIA runGenesis; "Run Genesis scoring tests."
  ARGS: ...dirs : String; "Test vector directories." ]
-- stf (jarstf) kept as separate exe — bless-mode server, not a test

def runAll (_ : Parsed) : IO UInt32 := do
  let tests : List (String × IO UInt32) := [
    ("crypto", cryptoTestMain *> pure 0),
    ("safrole", safroleJsonMain []),
    ("history", historyJsonMain []),
    ("statistics", statisticsJsonMain []),
    ("authorizations", authorizationsJsonMain []),
    ("disputes", disputesJsonMain []),
    ("preimages", preimagesJsonMain []),
    ("assurances", assurancesJsonMain []),
    ("reports", reportsJsonMain []),
    ("accumulate", accumulateJsonMain []),
    ("property", propertyMain),
    ("trie", trieTestMain),
    ("shuffle", shuffleTestMain),
    ("codec", codecTestMain []),
    ("block", blockTestMain []),
    ("erasure", erasureTestMain),
    ("genesis", genesisJsonMain [])
  ]
  let mut fail := 0
  for (name, test) in tests do
    IO.println s!"── {name} ──"
    let code ← test
    if code != 0 then fail := fail + 1
  if fail > 0 then
    IO.println s!"\n{fail} test suite(s) failed"
    return 1
  else
    IO.println "\nAll test suites passed"
    return 0

def jartest := `[Cli|
  jartest VIA runAll; ["0.1.0"]
  "JAR specification test runner. Without subcommand, runs all tests."

  SUBCOMMANDS:
    safroleCmd; historyCmd; statisticsCmd; authorizationsCmd;
    disputesCmd; preimagesCmd; assurancesCmd; reportsCmd;
    accumulateCmd; propertyCmd; trieCmd; shuffleCmd;
    codecCmd; blockCmd; erasureCmd; cryptoCmd; genesisCmd
]

def main (args : List String) : IO UInt32 :=
  jartest.validate args
