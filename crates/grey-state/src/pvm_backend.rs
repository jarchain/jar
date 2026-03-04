//! PVM backend — thin wrapper around grey-pvm.
//!
//! Exposes `PvmInstance` and `ExitReason` for use by the accumulate
//! sub-transition, implemented directly from the Gray Paper (Appendix A).

use grey_types::Gas;

pub use grey_pvm::ExitReason;

/// PVM instance backed by our grey-pvm implementation.
pub struct PvmInstance {
    inner: grey_pvm::vm::Pvm,
}

impl PvmInstance {
    /// Create a PVM from a code blob, arguments, and gas budget.
    pub fn initialize(code_blob: &[u8], args: &[u8], gas: Gas) -> Option<Self> {
        grey_pvm::program::initialize_program(code_blob, args, gas)
            .map(|pvm| PvmInstance { inner: pvm })
    }

    /// Run until exit (halt, panic, OOG, page fault, or host call).
    pub fn run(&mut self) -> ExitReason {
        let (reason, _) = self.inner.run();
        reason
    }

    pub fn gas(&self) -> Gas {
        self.inner.gas
    }
    pub fn set_gas(&mut self, gas: Gas) {
        self.inner.gas = gas;
    }

    pub fn pc(&self) -> u32 {
        self.inner.pc
    }
    pub fn set_pc(&mut self, pc: u32) {
        self.inner.pc = pc;
    }

    pub fn reg(&self, index: usize) -> u64 {
        self.inner.registers[index]
    }
    pub fn set_reg(&mut self, index: usize, value: u64) {
        self.inner.registers[index] = value;
    }

    pub fn read_byte(&self, addr: u32) -> Option<u8> {
        self.inner.memory.read_u8(addr)
    }

    pub fn write_byte(&mut self, addr: u32, value: u8) {
        self.inner.memory.write_u8(addr, value);
    }

    pub fn read_bytes(&self, addr: u32, len: u32) -> Vec<u8> {
        (0..len)
            .map(|i| self.inner.memory.read_u8(addr + i).unwrap_or(0))
            .collect()
    }

    /// Try to read bytes; returns None on page fault (any inaccessible byte).
    /// Used by host calls where inaccessible memory causes a PANIC.
    pub fn try_read_bytes(&self, addr: u32, len: u32) -> Option<Vec<u8>> {
        self.inner.memory.read_bytes(addr, len)
    }

    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.inner.memory.write_u8(addr + i as u32, byte);
        }
    }

    /// Try to write bytes; returns None on page fault (any non-writable byte).
    /// Used by host calls where non-writable memory causes a PANIC.
    pub fn try_write_bytes(&mut self, addr: u32, data: &[u8]) -> Option<()> {
        use grey_pvm::memory::MemoryAccess;
        for (i, &byte) in data.iter().enumerate() {
            match self.inner.memory.write_u8(addr.wrapping_add(i as u32), byte) {
                MemoryAccess::Ok => {}
                MemoryAccess::PageFault(_) => return None,
            }
        }
        Some(())
    }

    /// Enable instruction trace collection.
    pub fn enable_tracing(&mut self) {
        self.inner.tracing_enabled = true;
    }

    /// Dump code blob and bitmask to files for disassembly.
    pub fn dump_code(&self, code_path: &str, bitmask_path: &str) {
        let _ = std::fs::write(code_path, &self.inner.code);
        let _ = std::fs::write(bitmask_path, &self.inner.bitmask);
    }

    /// Take the collected instruction trace.
    pub fn take_trace(&mut self) -> Vec<(u32, u8)> {
        std::mem::take(&mut self.inner.pc_trace)
    }
}
