import Jar.Test.History

/-! Auto-generated history test vectors. Do not edit. -/

namespace Jar.Test.HistoryVectors

open Jar Jar.Test.History

def hexToBytes (s : String) : ByteArray :=
  let chars := s.toList
  let nibble (c : Char) : UInt8 :=
    if c.toNat >= 48 && c.toNat <= 57 then (c.toNat - 48).toUInt8
    else if c.toNat >= 97 && c.toNat <= 102 then (c.toNat - 87).toUInt8
    else if c.toNat >= 65 && c.toNat <= 70 then (c.toNat - 55).toUInt8
    else 0
  let rec go (cs : List Char) (acc : ByteArray) : ByteArray :=
    match cs with
    | hi :: lo :: rest => go rest (acc.push ((nibble hi <<< 4) ||| nibble lo))
    | _ => acc
  go chars ByteArray.empty

def hexSeq (s : String) : OctetSeq n := ⟨hexToBytes s, sorry⟩

-- ============================================================================
-- progress_blocks_history-1.json
-- ============================================================================

def progress_blocks_history_1_pre_history : Array HistoryEntry := #[]

def progress_blocks_history_1_pre_peaks : Array (Option Hash) := #[]

def progress_blocks_history_1_pre : FlatHistoryState := {
  history := progress_blocks_history_1_pre_history,
  mmrPeaks := progress_blocks_history_1_pre_peaks
}

def progress_blocks_history_1_post_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] }]

def progress_blocks_history_1_post_peaks : Array (Option Hash) := #[some (hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842")]

def progress_blocks_history_1_post : FlatHistoryState := {
  history := progress_blocks_history_1_post_history,
  mmrPeaks := progress_blocks_history_1_post_peaks
}

def progress_blocks_history_1_input : HistoryInput := {
  headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
  parentStateRoot := hexSeq "0e6c6cbf80b5fb00175001f7b0966bf1af83ff4406ede84f29a666a0fcbac801",
  accumulateRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
  workPackages := #[
    { hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" },
    { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }]
}

-- ============================================================================
-- progress_blocks_history-2.json
-- ============================================================================

def progress_blocks_history_2_pre_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] }]

def progress_blocks_history_2_pre_peaks : Array (Option Hash) := #[some (hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842")]

def progress_blocks_history_2_pre : FlatHistoryState := {
  history := progress_blocks_history_2_pre_history,
  mmrPeaks := progress_blocks_history_2_pre_peaks
}

def progress_blocks_history_2_post_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "1831dde64e40bfd8639c2d122e5ac00fe133c48cd16e1621ca6d5cf0b8e10d3b",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] },
    { headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
       beefyRoot := hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }] }]

def progress_blocks_history_2_post_peaks : Array (Option Hash) := #[none, some (hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b")]

def progress_blocks_history_2_post : FlatHistoryState := {
  history := progress_blocks_history_2_post_history,
  mmrPeaks := progress_blocks_history_2_post_peaks
}

def progress_blocks_history_2_input : HistoryInput := {
  headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
  parentStateRoot := hexSeq "1831dde64e40bfd8639c2d122e5ac00fe133c48cd16e1621ca6d5cf0b8e10d3b",
  accumulateRoot := hexSeq "7507515a48439dc58bc318c48a120b656136699f42bfd2bd45473becba53462d",
  workPackages := #[
    { hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }]
}

-- ============================================================================
-- progress_blocks_history-3.json
-- ============================================================================

def progress_blocks_history_3_pre_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "1831dde64e40bfd8639c2d122e5ac00fe133c48cd16e1621ca6d5cf0b8e10d3b",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] },
    { headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
       beefyRoot := hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b",
       stateRoot := hexSeq "f9ca27d76e5daadae15ca8e36f05a234dcb19855d53edb66878dc857a678751c",
       reported := #[{ hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }] },
    { headerHash := hexSeq "c51005ebb96f1f6485d25e1ced6bed9c8443530de3a319c01ba12b2447905b7b",
       beefyRoot := hexSeq "f3a933d781d1cdf941d8a94a9429ded7368e084f365decc334c22c7223053bc0",
       stateRoot := hexSeq "47163fc0722d6b6e83437f1345a3f62e1dfe02e7b41d39cb4ff2a6f3bd120b21",
       reported := #[] },
    { headerHash := hexSeq "f270b4d14f179593fb13ef13f9999fc81ecdd1664f4f143d3fdc609f8c970990",
       beefyRoot := hexSeq "e17766e385ad36f22ff2357053ab8af6a6335331b90de2aa9c12ec9f397fa414",
       stateRoot := hexSeq "09f858e15ae2d3a820166135d850b46f7d6f5df2719f96c5546007388811334a",
       reported := #[] },
    { headerHash := hexSeq "24cca5dbb31594e81fee2c10266d65cc8f5184e841fd5f0992980f74b036ab19",
       beefyRoot := hexSeq "5e3459175cf00bfc43b25c2b876149e65161a697894d94ec360e3407ca96b05f",
       stateRoot := hexSeq "99b3dd375039f0f03844625d3cde24d288d6b2e21fbe533646d91b1a5fb12719",
       reported := #[] },
    { headerHash := hexSeq "f9667c1f2eee903bb96d130aeda4655887acc50ff12071b4aa6cc3c65e9ba96a",
       beefyRoot := hexSeq "33be1be919c1b4c6367e089641d41d709836256265a543992fc9c1a3e1cd2d2f",
       stateRoot := hexSeq "cfa88eb0966a61f0e7fffed57f7b004a4fffc51f9b40b6ae68db08ad5a61a39d",
       reported := #[] },
    { headerHash := hexSeq "e9209ab342ae35c60e9cb755e4e169bbf5d9ed3b85a1a65c770535a6f0ed1981",
       beefyRoot := hexSeq "2eda798f51b0143cec40ef0a653fae185f080f3432580e99d8f17607fc59d787",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[] }]

def progress_blocks_history_3_pre_peaks : Array (Option Hash) := #[some (hexSeq "f986bfeff7411437ca6a23163a96b5582e6739f261e697dc6f3c05a1ada1ed0c"), some (hexSeq "ca29f72b6d40cfdb5814569cf906b3d369ae5f56b63d06f2b6bb47be191182a6"), some (hexSeq "e17766e385ad36f22ff2357053ab8af6a6335331b90de2aa9c12ec9f397fa414")]

def progress_blocks_history_3_pre : FlatHistoryState := {
  history := progress_blocks_history_3_pre_history,
  mmrPeaks := progress_blocks_history_3_pre_peaks
}

def progress_blocks_history_3_post_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "1831dde64e40bfd8639c2d122e5ac00fe133c48cd16e1621ca6d5cf0b8e10d3b",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] },
    { headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
       beefyRoot := hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b",
       stateRoot := hexSeq "f9ca27d76e5daadae15ca8e36f05a234dcb19855d53edb66878dc857a678751c",
       reported := #[{ hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }] },
    { headerHash := hexSeq "c51005ebb96f1f6485d25e1ced6bed9c8443530de3a319c01ba12b2447905b7b",
       beefyRoot := hexSeq "f3a933d781d1cdf941d8a94a9429ded7368e084f365decc334c22c7223053bc0",
       stateRoot := hexSeq "47163fc0722d6b6e83437f1345a3f62e1dfe02e7b41d39cb4ff2a6f3bd120b21",
       reported := #[] },
    { headerHash := hexSeq "f270b4d14f179593fb13ef13f9999fc81ecdd1664f4f143d3fdc609f8c970990",
       beefyRoot := hexSeq "e17766e385ad36f22ff2357053ab8af6a6335331b90de2aa9c12ec9f397fa414",
       stateRoot := hexSeq "09f858e15ae2d3a820166135d850b46f7d6f5df2719f96c5546007388811334a",
       reported := #[] },
    { headerHash := hexSeq "24cca5dbb31594e81fee2c10266d65cc8f5184e841fd5f0992980f74b036ab19",
       beefyRoot := hexSeq "5e3459175cf00bfc43b25c2b876149e65161a697894d94ec360e3407ca96b05f",
       stateRoot := hexSeq "99b3dd375039f0f03844625d3cde24d288d6b2e21fbe533646d91b1a5fb12719",
       reported := #[] },
    { headerHash := hexSeq "f9667c1f2eee903bb96d130aeda4655887acc50ff12071b4aa6cc3c65e9ba96a",
       beefyRoot := hexSeq "33be1be919c1b4c6367e089641d41d709836256265a543992fc9c1a3e1cd2d2f",
       stateRoot := hexSeq "cfa88eb0966a61f0e7fffed57f7b004a4fffc51f9b40b6ae68db08ad5a61a39d",
       reported := #[] },
    { headerHash := hexSeq "e9209ab342ae35c60e9cb755e4e169bbf5d9ed3b85a1a65c770535a6f0ed1981",
       beefyRoot := hexSeq "2eda798f51b0143cec40ef0a653fae185f080f3432580e99d8f17607fc59d787",
       stateRoot := hexSeq "8a812d298cde0b1d69bc0a2b32a7a36eb5dfff3dd7b20feca8e7087b447eee41",
       reported := #[] },
    { headerHash := hexSeq "214facca26763b878b35a9fe988d3b0dd11428d17db1a56d743d678619ce3a08",
       beefyRoot := hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "3d6e543fc243dbc082fc7768d5ec3050e2bf2f69389ef225ddacbfbb5e95d450", exportsRoot := hexSeq "4fd3420ccf26786008a14a282f28ff1dc28413d7b602645eac8aaa921688c370" }] }]

def progress_blocks_history_3_post_peaks : Array (Option Hash) := #[none, none, none, some (hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6")]

def progress_blocks_history_3_post : FlatHistoryState := {
  history := progress_blocks_history_3_post_history,
  mmrPeaks := progress_blocks_history_3_post_peaks
}

def progress_blocks_history_3_input : HistoryInput := {
  headerHash := hexSeq "214facca26763b878b35a9fe988d3b0dd11428d17db1a56d743d678619ce3a08",
  parentStateRoot := hexSeq "8a812d298cde0b1d69bc0a2b32a7a36eb5dfff3dd7b20feca8e7087b447eee41",
  accumulateRoot := hexSeq "8223d5eaa57ccef85993b7180a593577fd38a65fb41e4bcea2933d8b202905f0",
  workPackages := #[
    { hash := hexSeq "3d6e543fc243dbc082fc7768d5ec3050e2bf2f69389ef225ddacbfbb5e95d450", exportsRoot := hexSeq "4fd3420ccf26786008a14a282f28ff1dc28413d7b602645eac8aaa921688c370" }]
}

-- ============================================================================
-- progress_blocks_history-4.json
-- ============================================================================

def progress_blocks_history_4_pre_history : Array HistoryEntry := #[
    { headerHash := hexSeq "530ef4636fedd498e99c7601581271894a53e965e901e8fa49581e525f165dae",
       beefyRoot := hexSeq "8720b97ddd6acc0f6eb66e095524038675a4e4067adc10ec39939eaefc47d842",
       stateRoot := hexSeq "1831dde64e40bfd8639c2d122e5ac00fe133c48cd16e1621ca6d5cf0b8e10d3b",
       reported := #[{ hash := hexSeq "016cb55eb7b84e0d495d40832c7238965baeb468932c415dc2ceffe0afb039e5", exportsRoot := hexSeq "935f6dfef36fa06e10a9ba820f933611c05c06a207b07141fe8d87465870c11c" }, { hash := hexSeq "76bcb24901299c331f0ca7342f4874f19b213ee72df613d50699e7e25edb82a6", exportsRoot := hexSeq "c825d16b7325ca90287123bd149d47843c999ce686ed51eaf8592dd2759272e3" }] },
    { headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
       beefyRoot := hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b",
       stateRoot := hexSeq "f9ca27d76e5daadae15ca8e36f05a234dcb19855d53edb66878dc857a678751c",
       reported := #[{ hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }] },
    { headerHash := hexSeq "c51005ebb96f1f6485d25e1ced6bed9c8443530de3a319c01ba12b2447905b7b",
       beefyRoot := hexSeq "f3a933d781d1cdf941d8a94a9429ded7368e084f365decc334c22c7223053bc0",
       stateRoot := hexSeq "47163fc0722d6b6e83437f1345a3f62e1dfe02e7b41d39cb4ff2a6f3bd120b21",
       reported := #[] },
    { headerHash := hexSeq "f270b4d14f179593fb13ef13f9999fc81ecdd1664f4f143d3fdc609f8c970990",
       beefyRoot := hexSeq "e17766e385ad36f22ff2357053ab8af6a6335331b90de2aa9c12ec9f397fa414",
       stateRoot := hexSeq "09f858e15ae2d3a820166135d850b46f7d6f5df2719f96c5546007388811334a",
       reported := #[] },
    { headerHash := hexSeq "24cca5dbb31594e81fee2c10266d65cc8f5184e841fd5f0992980f74b036ab19",
       beefyRoot := hexSeq "5e3459175cf00bfc43b25c2b876149e65161a697894d94ec360e3407ca96b05f",
       stateRoot := hexSeq "99b3dd375039f0f03844625d3cde24d288d6b2e21fbe533646d91b1a5fb12719",
       reported := #[] },
    { headerHash := hexSeq "f9667c1f2eee903bb96d130aeda4655887acc50ff12071b4aa6cc3c65e9ba96a",
       beefyRoot := hexSeq "33be1be919c1b4c6367e089641d41d709836256265a543992fc9c1a3e1cd2d2f",
       stateRoot := hexSeq "cfa88eb0966a61f0e7fffed57f7b004a4fffc51f9b40b6ae68db08ad5a61a39d",
       reported := #[] },
    { headerHash := hexSeq "e9209ab342ae35c60e9cb755e4e169bbf5d9ed3b85a1a65c770535a6f0ed1981",
       beefyRoot := hexSeq "2eda798f51b0143cec40ef0a653fae185f080f3432580e99d8f17607fc59d787",
       stateRoot := hexSeq "8a812d298cde0b1d69bc0a2b32a7a36eb5dfff3dd7b20feca8e7087b447eee41",
       reported := #[] },
    { headerHash := hexSeq "214facca26763b878b35a9fe988d3b0dd11428d17db1a56d743d678619ce3a08",
       beefyRoot := hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "3d6e543fc243dbc082fc7768d5ec3050e2bf2f69389ef225ddacbfbb5e95d450", exportsRoot := hexSeq "4fd3420ccf26786008a14a282f28ff1dc28413d7b602645eac8aaa921688c370" }] }]

def progress_blocks_history_4_pre_peaks : Array (Option Hash) := #[none, none, none, some (hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6")]

def progress_blocks_history_4_pre : FlatHistoryState := {
  history := progress_blocks_history_4_pre_history,
  mmrPeaks := progress_blocks_history_4_pre_peaks
}

def progress_blocks_history_4_post_history : Array HistoryEntry := #[
    { headerHash := hexSeq "241d129c6edc2114e6dfba7d556f7f7c66399b55ceec3078a53d44c752ba7e9a",
       beefyRoot := hexSeq "7076c31882a5953e097aef8378969945e72807c4705e53a0c5aacc9176f0d56b",
       stateRoot := hexSeq "f9ca27d76e5daadae15ca8e36f05a234dcb19855d53edb66878dc857a678751c",
       reported := #[{ hash := hexSeq "3cc8d8c94e7b3ee01e678c63fd6b5db894fc807dff7fe10a11ab41e70194894d", exportsRoot := hexSeq "c0edfe377d20b9f4ed7d9df9511ef904c87e24467364f0f7f75f20cfe90dd8fb" }] },
    { headerHash := hexSeq "c51005ebb96f1f6485d25e1ced6bed9c8443530de3a319c01ba12b2447905b7b",
       beefyRoot := hexSeq "f3a933d781d1cdf941d8a94a9429ded7368e084f365decc334c22c7223053bc0",
       stateRoot := hexSeq "47163fc0722d6b6e83437f1345a3f62e1dfe02e7b41d39cb4ff2a6f3bd120b21",
       reported := #[] },
    { headerHash := hexSeq "f270b4d14f179593fb13ef13f9999fc81ecdd1664f4f143d3fdc609f8c970990",
       beefyRoot := hexSeq "e17766e385ad36f22ff2357053ab8af6a6335331b90de2aa9c12ec9f397fa414",
       stateRoot := hexSeq "09f858e15ae2d3a820166135d850b46f7d6f5df2719f96c5546007388811334a",
       reported := #[] },
    { headerHash := hexSeq "24cca5dbb31594e81fee2c10266d65cc8f5184e841fd5f0992980f74b036ab19",
       beefyRoot := hexSeq "5e3459175cf00bfc43b25c2b876149e65161a697894d94ec360e3407ca96b05f",
       stateRoot := hexSeq "99b3dd375039f0f03844625d3cde24d288d6b2e21fbe533646d91b1a5fb12719",
       reported := #[] },
    { headerHash := hexSeq "f9667c1f2eee903bb96d130aeda4655887acc50ff12071b4aa6cc3c65e9ba96a",
       beefyRoot := hexSeq "33be1be919c1b4c6367e089641d41d709836256265a543992fc9c1a3e1cd2d2f",
       stateRoot := hexSeq "cfa88eb0966a61f0e7fffed57f7b004a4fffc51f9b40b6ae68db08ad5a61a39d",
       reported := #[] },
    { headerHash := hexSeq "e9209ab342ae35c60e9cb755e4e169bbf5d9ed3b85a1a65c770535a6f0ed1981",
       beefyRoot := hexSeq "2eda798f51b0143cec40ef0a653fae185f080f3432580e99d8f17607fc59d787",
       stateRoot := hexSeq "8a812d298cde0b1d69bc0a2b32a7a36eb5dfff3dd7b20feca8e7087b447eee41",
       reported := #[] },
    { headerHash := hexSeq "214facca26763b878b35a9fe988d3b0dd11428d17db1a56d743d678619ce3a08",
       beefyRoot := hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6",
       stateRoot := hexSeq "a6aae15dfd6389e8f18e72a9dd6c03071e73c9a7f47df27415aaca0de068cb50",
       reported := #[{ hash := hexSeq "3d6e543fc243dbc082fc7768d5ec3050e2bf2f69389ef225ddacbfbb5e95d450", exportsRoot := hexSeq "4fd3420ccf26786008a14a282f28ff1dc28413d7b602645eac8aaa921688c370" }] },
    { headerHash := hexSeq "ad6862875431e427df25066819b82c648cf0c0d920904d58391a36a95bd9d481",
       beefyRoot := hexSeq "ebdb6db060afceaa2a99a499a84476847444ffc3787f6a4786e713f5362dbf4d",
       stateRoot := hexSeq "0000000000000000000000000000000000000000000000000000000000000000",
       reported := #[{ hash := hexSeq "1b03bc6eda0326c35df1b3f80fb1590016d29e1e9cef9b0b35853a1f6d069d7f", exportsRoot := hexSeq "7d06ce0167ea77740512095c9f269f391ca620aa609a509fd5c979a5c0bfd4c0" }] }]

def progress_blocks_history_4_post_peaks : Array (Option Hash) := #[some (hexSeq "a983417440b618f29ed0b7fa65212fce2d363cb2b2c18871a05c4f67217290b0"), none, none, some (hexSeq "658b919f734bd39262c10589aa1afc657471d902a6a361c044f78de17d660bc6")]

def progress_blocks_history_4_post : FlatHistoryState := {
  history := progress_blocks_history_4_post_history,
  mmrPeaks := progress_blocks_history_4_post_peaks
}

def progress_blocks_history_4_input : HistoryInput := {
  headerHash := hexSeq "ad6862875431e427df25066819b82c648cf0c0d920904d58391a36a95bd9d481",
  parentStateRoot := hexSeq "a6aae15dfd6389e8f18e72a9dd6c03071e73c9a7f47df27415aaca0de068cb50",
  accumulateRoot := hexSeq "a983417440b618f29ed0b7fa65212fce2d363cb2b2c18871a05c4f67217290b0",
  workPackages := #[
    { hash := hexSeq "1b03bc6eda0326c35df1b3f80fb1590016d29e1e9cef9b0b35853a1f6d069d7f", exportsRoot := hexSeq "7d06ce0167ea77740512095c9f269f391ca620aa609a509fd5c979a5c0bfd4c0" }]
}

-- ============================================================================
-- Test Runner
-- ============================================================================

end Jar.Test.HistoryVectors

open Jar.Test.History Jar.Test.HistoryVectors in
def main : IO Unit := do
  IO.println "Running history test vectors..."
  let mut passed := (0 : Nat)
  let mut failed := (0 : Nat)
  if (← runTest "progress_blocks_history_1" progress_blocks_history_1_pre progress_blocks_history_1_input progress_blocks_history_1_post)
  then passed := passed + 1
  else failed := failed + 1
  if (← runTest "progress_blocks_history_2" progress_blocks_history_2_pre progress_blocks_history_2_input progress_blocks_history_2_post)
  then passed := passed + 1
  else failed := failed + 1
  if (← runTest "progress_blocks_history_3" progress_blocks_history_3_pre progress_blocks_history_3_input progress_blocks_history_3_post)
  then passed := passed + 1
  else failed := failed + 1
  if (← runTest "progress_blocks_history_4" progress_blocks_history_4_pre progress_blocks_history_4_input progress_blocks_history_4_post)
  then passed := passed + 1
  else failed := failed + 1
  IO.println s!"History: {passed} passed, {failed} failed out of 4"
  if failed > 0 then
    IO.Process.exit 1
