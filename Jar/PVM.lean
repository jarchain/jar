/-!
# Polkadot Virtual Machine — Appendix A

- RISC-V rv64em based ISA, 13 registers (64-bit)
- Pageable RAM: 32-bit addressable, 4096-byte pages
- Instruction set (~150 opcodes)
- Gas metering
- Exit reasons: halt, panic, OOG, page fault, host-call
- Standard initialization `Y(p, a)` (A.37–A.43)
- Invocation contexts: `Ψ_I`, `Ψ_R`, `Ψ_A` (Appendix B)
- Host-call dispatch `Ψ_H` (A.36)
-/
