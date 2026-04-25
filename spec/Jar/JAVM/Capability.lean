import Jar.JAVM

/-!
# JAVM Capability Types

Capability-based execution model for the jar1 variant. Defines five
program capability types (UNTYPED, DATA, CODE, HANDLE, CALLABLE) and
the cap table, VM state machine, ecalli/ecall dispatch, and capability
indirection.

This module defines the data structures only. Execution logic is in
`Jar.JAVM.Kernel`.
-/

namespace Jar.JAVM.Cap

-- ============================================================================
-- Capability Types
-- ============================================================================

/-- Memory access mode, set at MAP time. -/
inductive Access where
  /-- Read-only. -/
  | ro : Access
  /-- Read-write. -/
  | rw : Access
  deriving BEq, Inhabited, Repr

/-- Cap entry type in the blob manifest. -/
inductive ManifestCapType where
  /-- Code capability (Harvard architecture, not in data address space). -/
  | code : ManifestCapType
  /-- Data capability (physical pages with exclusive mapping). -/
  | data : ManifestCapType
  deriving BEq, Inhabited

/-- DATA capability: physical pages with exclusive mapping and per-page bitmap.

Move-only. Each DATA cap has a single base_offset (set on first MAP) and a
per-page mapped bitmap tracking which pages are present in the address space.
Page P maps to address `base_offset + P * 4096`. -/
structure DataCap where
  /-- Offset into the backing memfd (in pages). -/
  backingOffset : Nat
  /-- Number of pages. -/
  pageCount : Nat
  /-- Base offset in address space (set on first MAP, fixed thereafter). None = unmapped. -/
  baseOffset : Option Nat := none
  /-- Access mode (set on first MAP, fixed thereafter). -/
  access : Option Access := none
  /-- Per-page mapped bitmap. True = page present in address space. -/
  mappedBitmap : Array Bool := #[]
  deriving Inhabited

/-- UNTYPED capability: bump allocator. Copyable (shared offset). -/
structure UntypedCap where
  /-- Current bump offset (in pages). -/
  offset : Nat
  /-- Total pages available. -/
  total : Nat
  deriving Inhabited

/-- CODE capability: compiled PVM code. Copyable. -/
structure CodeCap where
  /-- Unique identifier within invocation. -/
  id : Nat
  deriving Inhabited, BEq

/-- HANDLE capability: VM owner. Unique, not copyable.

Provides CALL (run VM) plus management ops via ecall:
DOWNGRADE, SET_MAX_GAS, DIRTY, RESUME. -/
structure HandleCap where
  /-- VM index in the kernel's VM pool. -/
  vmId : Nat
  /-- Per-CALL gas ceiling (inherited by DOWNGRADEd CALLABLEs). -/
  maxGas : Option Nat := none
  deriving Inhabited

/-- CALLABLE capability: VM entry point. Copyable. -/
structure CallableCap where
  /-- VM index in the kernel's VM pool. -/
  vmId : Nat
  /-- Per-CALL gas ceiling. -/
  maxGas : Option Nat := none
  deriving Inhabited

/-- Protocol capability: kernel-handled, replaceable with CALLABLE. -/
structure ProtocolCap where
  /-- Protocol cap ID. -/
  id : Nat
  deriving Inhabited, BEq

/-- A capability in the cap table. -/
inductive Cap where
  /-- UNTYPED: bump allocator for physical page allocation. Copyable. -/
  | untyped (u : UntypedCap) : Cap
  /-- DATA: physical pages with exclusive mapping. Move-only. -/
  | data (d : DataCap) : Cap
  /-- CODE: compiled PVM bytecode (Harvard architecture). Copyable. -/
  | code (c : CodeCap) : Cap
  /-- HANDLE: unique VM owner with management ops. Move-only. -/
  | handle (h : HandleCap) : Cap
  /-- CALLABLE: VM entry point (CALL only). Copyable. -/
  | callable (c : CallableCap) : Cap
  /-- Protocol: kernel-handled service (storage, preimages, etc.). Copyable. -/
  | protocol (p : ProtocolCap) : Cap
  deriving Inhabited

/-- Whether a capability type supports COPY. -/
def Cap.isCopyable : Cap → Bool
  | .untyped _ => true
  | .code _ => true
  | .callable _ => true
  | .protocol _ => true
  | .data _ => false
  | .handle _ => false

/-- Create a copy of this cap (only for copyable types). -/
def Cap.tryCopy : Cap → Option Cap
  | .untyped u => some (.untyped u)
  | .code c => some (.code c)
  | .callable c => some (.callable c)
  | .protocol p => some (.protocol p)
  | .data _ => none
  | .handle _ => none

-- ============================================================================
-- Cap Table (CNode)
-- ============================================================================

/-- IPC slot index. CALL on slot 0 = REPLY. -/
def ipcSlot : Nat := 0

/-- Cap table: 256 slots indexed by u8. Each VM's cap table is a CNode.

The original bitmap tracks which protocol cap slots are unmodified
(for compiler fast-path inlining of ecalli on protocol caps). -/
structure CapTable where
  /-- 256 capability slots indexed by u8. -/
  slots : Array (Option Cap)
  /-- Per-slot original bitmap (256 bits). True = slot holds original
  kernel-populated protocol cap. Set to false on DROP, MOVE-in, or MOVE-out. -/
  originalBitmap : Array Bool
  deriving Inhabited

namespace CapTable

/-- Create an empty cap table with all 256 slots unoccupied. -/
def empty : CapTable :=
  { slots := Array.replicate 256 none
    originalBitmap := Array.replicate 256 false }

/-- Get the cap at the given slot index, or none if empty/out of bounds. -/
def get (t : CapTable) (idx : Nat) : Option Cap :=
  if idx < t.slots.size then t.slots[idx]! else none

/-- Set a cap at the given slot index, clearing the original flag for protocol slots. -/
def set (t : CapTable) (idx : Nat) (c : Cap) : CapTable :=
  if idx < t.slots.size then
    { slots := t.slots.set! idx (some c)
      originalBitmap := if idx < 29 then t.originalBitmap.set! idx false
                        else t.originalBitmap }
  else t

/-- Set a cap and mark it as original (for kernel init of protocol caps). -/
def setOriginal (t : CapTable) (idx : Nat) (c : Cap) : CapTable :=
  if idx < t.slots.size then
    { slots := t.slots.set! idx (some c)
      originalBitmap := if idx < t.originalBitmap.size then t.originalBitmap.set! idx true
                        else t.originalBitmap }
  else t

/-- Remove and return the cap at the given slot, clearing the original flag. -/
def take (t : CapTable) (idx : Nat) : CapTable × Option Cap :=
  if idx < t.slots.size then
    let c := t.slots[idx]!
    ({ slots := t.slots.set! idx none
       originalBitmap := if idx < 29 then t.originalBitmap.set! idx false
                         else t.originalBitmap }, c)
  else (t, none)

/-- Check if a slot is empty (no cap). -/
def isEmpty (t : CapTable) (idx : Nat) : Bool :=
  if idx < t.slots.size then t.slots[idx]!.isNone else true

end CapTable

-- ============================================================================
-- Capability Indirection
-- ============================================================================

/-- Indirection encoding: u32 byte-packed HANDLE chain.

```
byte 0: target cap slot (0-255)
byte 1: indirection level 0 (0x00 = end, 1-255 = HANDLE slot)
byte 2: indirection level 1 (0x00 = end, 1-255 = HANDLE slot)
byte 3: indirection level 2 (0x00 = end, 1-255 = HANDLE slot)
```

Slot 0 (IPC) cannot be used for indirection. `(u8 as u32)` = local slot. -/
def CapRef := UInt32

/-- Maximum indirection depth (3 levels). -/
def maxIndirectionDepth : Nat := 3

-- ============================================================================
-- VM State Machine
-- ============================================================================

/-- VM lifecycle states.

FAULTED is non-terminal: RESUME can restart a faulted VM,
preserving registers and PC (retries the faulting instruction). -/
inductive VmState where
  /-- Idle: waiting to be CALLed. Non-terminal. -/
  | idle : VmState
  /-- Running: currently executing instructions. -/
  | running : VmState
  /-- Waiting for REPLY: blocked at a CALL instruction. -/
  | waitingForReply : VmState
  /-- Halted: clean exit. Terminal state. -/
  | halted : VmState
  /-- Faulted: panic, OOG, or page fault. Non-terminal (RESUMEable). -/
  | faulted : VmState
  deriving BEq, Inhabited, Repr

/-- A single VM instance. -/
structure VmInstance where
  /-- Current lifecycle state (idle, running, waiting, halted, faulted). -/
  state : VmState
  /-- CODE cap identifier this VM was created from. -/
  codeCapId : Nat
  /-- 13 general-purpose 64-bit registers (phi[0..12]). -/
  registers : JAVM.Registers
  /-- Program counter. -/
  pc : Nat
  /-- Cap table (256-slot CNode). -/
  capTable : CapTable
  /-- Parent VM index for REPLY routing. None for root VM. -/
  caller : Option Nat
  /-- Entry point index in the CODE cap's jump table. -/
  entryIndex : Nat
  /-- Remaining gas. -/
  gas : Nat
  deriving Inhabited

/-- Call frame saved on the kernel's call stack. -/
structure CallFrame where
  /-- VM index of the caller (for REPLY routing). -/
  callerVmId : Nat
  /-- IPC cap slot index in the callee's cap table. -/
  ipcCapIdx : Option Nat
  /-- Whether the IPC cap was mapped before the CALL (for restore on REPLY). -/
  ipcWasMapped : Option (Nat × Access)
  deriving Inhabited

-- ============================================================================
-- ecalli Dispatch (CALL a cap)
-- ============================================================================

/-- ecalli immediate decoding. ecalli is CALL-only — subject cap from
the u32 immediate (with indirection encoding). Management ops use ecall. -/
inductive EcalliOp where
  /-- CALL cap at the resolved slot. -/
  | call (capRef : CapRef) : EcalliOp

/-- Decode an ecalli immediate. Always a CALL. -/
def decodeEcalli (imm : UInt32) : EcalliOp :=
  .call imm

-- ============================================================================
-- ecall Dispatch (Management ops + dynamic CALL)
-- ============================================================================

/-- ecall operation codes (from φ[11]).

Subject and object cap references are packed in φ[12] as two u32
values with indirection encoding: subject = low u32, object = high u32. -/
inductive EcallOp where
  /-- Dynamic CALL (same semantics as ecalli, dynamic subject). -/
  | call : EcallOp
  /-- MAP pages of a DATA cap in its CNode. -/
  | map : EcallOp
  /-- UNMAP pages of a DATA cap in its CNode. -/
  | unmap : EcallOp
  /-- SPLIT a DATA cap. -/
  | split : EcallOp
  /-- DROP (destroy) a cap. -/
  | drop : EcallOp
  /-- MOVE a cap between CNodes. -/
  | move : EcallOp
  /-- COPY a cap between CNodes (copyable types only). -/
  | copy : EcallOp
  /-- DOWNGRADE a HANDLE to CALLABLE. -/
  | downgrade : EcallOp
  /-- SET_MAX_GAS on a HANDLE. -/
  | setMaxGas : EcallOp
  /-- Read dirty bitmap of a child's DATA cap. -/
  | dirty : EcallOp
  /-- RESUME a FAULTED VM. -/
  | resume : EcallOp
  /-- Unknown/invalid op. -/
  | unknown : EcallOp

/-- Decode an ecall operation from φ[11]. -/
def decodeEcall (op : Nat) : EcallOp :=
  match op with
  | 0x00 => .call
  | 0x02 => .map
  | 0x03 => .unmap
  | 0x04 => .split
  | 0x05 => .drop
  | 0x06 => .move
  | 0x07 => .copy
  | 0x0A => .downgrade
  | 0x0B => .setMaxGas
  | 0x0C => .dirty
  | 0x0D => .resume
  | _ => .unknown

/-- Result of CALL dispatch. -/
inductive DispatchResult where
  /-- Continue execution of active VM. -/
  | continue_ : DispatchResult
  /-- Protocol cap called — host should handle. -/
  | protocolCall (slot : Nat) (regs : JAVM.Registers) (gas : Nat) : DispatchResult
  /-- Root VM halted normally. -/
  | rootHalt (value : Nat) : DispatchResult
  /-- Root VM panicked. -/
  | rootPanic : DispatchResult
  /-- Root VM out of gas. -/
  | rootOutOfGas : DispatchResult
  /-- Fault handled by parent (RESUME or cascade). -/
  | faultHandled : DispatchResult

-- ============================================================================
-- Protocol Cap Numbering (slots 1-28, IPC at slot 0)
-- ============================================================================

/-- Protocol cap IDs. Slot 0 = IPC (REPLY). Protocol caps at slots 1-28.
Gas remaining query is at slot 1 (protocolGas). -/
def protocolGas := 1
def protocolFetch := 2
def protocolPreimageLookup := 3
def protocolStorageR := 4
def protocolStorageW := 5
def protocolInfo := 6
def protocolHistorical := 7
def protocolExport := 8
def protocolCompile := 9
-- 10-14 reserved (was peek/poke/pages/invoke/expunge)
def protocolBless := 15
def protocolAssign := 16
def protocolDesignate := 17
def protocolCheckpoint := 18
def protocolServiceNew := 19
def protocolServiceUpgrade := 20
def protocolTransfer := 21
def protocolServiceEject := 22
def protocolPreimageQuery := 23
def protocolPreimageSolicit := 24
def protocolPreimageForget := 25
def protocolOutput := 26
def protocolPreimageProvide := 27
def protocolQuota := 28

-- ============================================================================
-- JAR Blob Format
-- ============================================================================

/-- JAR magic: 'J','A','R', 0x02. -/
def jarMagic : UInt32 := 0x02524148

/-- Capability manifest entry from the blob. -/
structure CapManifestEntry where
  /-- Cap slot index in the initial cap table. -/
  capIndex : Nat
  /-- Whether this is a CODE or DATA cap. -/
  capType : ManifestCapType
  /-- First page number in the address space. -/
  basePage : Nat
  /-- Number of pages this cap covers. -/
  pageCount : Nat
  /-- Initial access mode (ro or rw). -/
  initAccess : Access
  /-- Offset into the blob where cap data begins. -/
  dataOffset : Nat
  /-- Length of cap data in the blob. -/
  dataLen : Nat
  deriving Inhabited

/-- Parsed JAR header. -/
structure ProgramHeader where
  /-- Total memory pages to allocate for the program. -/
  memoryPages : Nat
  /-- Number of capability entries in the manifest. -/
  capCount : Nat
  /-- Cap slot index to CALL after initialization. -/
  invokeCap : Nat
  deriving Inhabited

-- ============================================================================
-- Limits
-- ============================================================================

/-- Maximum CODE caps per invocation. -/
def maxCodeCaps : Nat := 5

/-- Maximum VMs (HANDLEs) per invocation (u16 VM IDs). -/
def maxVms : Nat := 65535

/-- Gas cost per page for RETYPE. -/
def gasPerPage : Nat := 1500

end Jar.JAVM.Cap
