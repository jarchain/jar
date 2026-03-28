import Jar.Test.Properties
import Jar.Variant

open Jar

def propertyMain : IO UInt32 := do
  letI := JamVariant.gp072_tiny.toJamConfig
  Jar.Test.Properties.runAll
