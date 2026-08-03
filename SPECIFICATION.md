# VMR Specification (Corrected & Expanded)

All multi-byte integers (immediates, offsets, string-length prefixes) are
**big-endian**.

## 1. File Format

```
Offset 0x00-0x02   Magic:      0x76 0x6D 0x72   ("vmr", ASCII)
Offset 0x03        File type:  0x78 ('x') = Executable
                               0x6F ('o') = Object file
Offset 0x04..      Payload
```

### 1.1 Object File (`.vmo`) Payload

```
u16   export_count
  export_count × {
      u16      name_len
      u8[len]  name (UTF-8)
      u16      offset        -- offset of the label within this object's bytecode
  }
u16   external_relocation_count
  × {
      u16      name_len
      u8[len]  name
      u16      patch_offset  -- position in bytecode of a 2-byte address to patch
  }
u16   internal_relocation_count
  × {
      u16      patch_offset  -- position in bytecode of a 2-byte address to patch
                                 (patched by adding this object's link-time base address)
  }
u8[]  bytecode                -- remainder of the file
```

Labels that are _not_ exported are assembler-internal scratch data and are
**not** part of the on-disk format — only exports and relocations survive
serialization.

A local reference (`.label`) to a label that _is_ exported from the same file
is still routed through `external_relocations` rather than
`internal_relocations` (since exported names live in a separate table from
plain labels). It still resolves correctly at link time via the global symbol
table, just via a different code path than a plain local label would use.

### 1.2 Executable Payload

```
Offset 0x04         Opcode 0xB0 (JMP)
Offset 0x05-0x06    2-byte entry-point address (big-endian)
Offset 0x07..       Linked bytecode from all input objects, concatenated in load order
```

The linker always prepends this 3-byte `JMP <entry>` "trampoline" ahead of any
user code. Exported addresses recorded
in the linker's global symbol table are computed as
`3 + (bytes of all preceding objects' bytecode)` — i.e. addresses are relative
to the start of the code region (address `0` = the trampoline's own opcode
byte), not to the start of user code.

The linker rejects programs where the combined bytecode across all objects
exceeds `0xFFFF` bytes, and the runner tool double-checks the full output
(header included) against the same limit.

## 2. Instruction Format

```
1 byte OPCODE, then:
  nothing
  or REG
  or IMM
  or REG       | REG
  or REG       | IMM
  or IMM       | REG
  or IMM       | IMM
  or REG       | IMM  | REG
  or REG       | IMM  | IMM
```

`IMM` is always 2 bytes. A register operand is always 1 byte in
the instruction stream (an index 0x00–0x10) — this is distinct from the fact
that each register's _stored value_ is 2 bytes wide.

### 2.1 Argument-Type Codes

| Code | Name      | Shape           | Total instr. bytes (incl. opcode) |
| ---- | --------- | --------------- | --------------------------------- |
| 0x00 | None      | —               | 1                                 |
| 0x01 | Reg       | `REG`           | 2                                 |
| 0x02 | RegReg    | `REG, REG`      | 3                                 |
| 0x03 | RegImm    | `REG, IMM`      | 4                                 |
| 0x04 | ImmImm    | `IMM, IMM`      | 5                                 |
| 0x05 | RegImmReg | `REG, IMM, REG` | 5                                 |
| 0x06 | ImmReg    | `IMM, REG`      | 4                                 |
| 0x07 | Imm       | `IMM`           | 3                                 |
| 0x08 | RegImmImm | `REG, IMM, IMM` | 6                                 |

### 2.2 Opcode Reference

| Opcode                   | Value      | Args          | Notes                                                                                                                         |
| ------------------------ | ---------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------- | --- | ------------- | ---- | ------ | ------------------ |
| HALT                     | 0x00       | None          |                                                                                                                               |
| NOP                      | 0x01       | None          |                                                                                                                               |
| MOV                      | 0x10       | RegReg        | R1 ← R2                                                                                                                       |
| MOV_IMM                  | 0x11       | RegImm        |                                                                                                                               |
| LOAD                     | 0x20       | RegReg        | R1 ← Mem\[R2\]                                                                                                                |
| LOAD_REL                 | 0x21       | RegImmReg     | Mem\[R1 + offset\] → R3 (signed 16-bit offset)                                                                                |
| LOAD_IMM                 | 0x22       | ImmReg        | Mem\[imm\] → R2                                                                                                               |
| STORE_R_R                | 0x30       | RegReg        | Mem\[R1\] ← R2                                                                                                                |
| STORE_REL_R              | 0x31       | RegImmReg     | Mem\[R1 + offset\] ← R3                                                                                                       |
| STORE_IMM_R              | 0x32       | ImmReg        | Mem\[imm\] ← R2                                                                                                               |
| STORE_R_IMM              | 0x33       | RegImm        | Mem\[R1\] ← imm                                                                                                               |
| STORE_REL_IMM            | 0x34       | RegImmImm     | Mem\[R1 + offset\] ← imm                                                                                                      |     | STORE_IMM_IMM | 0x35 | ImmImm | Mem\[imm1\] ← imm2 |
| ADD / ADD_IMM            | 0x40/41    | RegReg/RegImm | sets E, C, N flags (see §3)                                                                                                   |
| SUB / SUB_IMM            | 0x42/43    | RegReg/RegImm | sets E, C, N; writes result back to R1                                                                                        |
| CMP / CMP_IMM            | 0x80/81    | RegReg/RegImm | same as SUB but discards result                                                                                               |
| AND/OR/XOR (+\_IMM)      | 0x50–0x55  | RegReg/RegImm | no flags affected                                                                                                             |
| DIV_MOD                  | 0x60       | Reg           | implicit dividend R1; divisor = operand reg; quotient→R0, remainder→R1 (overwrites the dividend). Errors on division by zero. |
| SHL/SHR (+\_IMM)         | 0x70–0x73  | RegReg/RegImm | no flags affected                                                                                                             |
| PUSH / PUSH_M / PUSH_IMM | 0x90/91/92 | Reg/Reg/Imm   | decrement RSP, then write                                                                                                     |
| POP / POP_M / POP_IMM    | 0xA0/A1/A2 | Reg/Reg/Imm   | read, then increment RSP                                                                                                      |
| JMP…JNC                  | 0xB0–0xBC  | Imm           | see §3                                                                                                                        |
| CALL                     | 0xD0       | Imm           | pushes return address (address _after_ CALL), jumps                                                                           |
| RET                      | 0xD2       | None          | pops return address into PC                                                                                                   |
| SYSCALL                  | 0xFF       | None          | operands come from registers, not the instruction stream — see §5                                                             |

Opcode ranges `0xC0-0xCF` and `0xD1` are currently unassigned/reserved.

## 3. Flags Register

```

bit 0 (0b0001) Equals
bit 1 (0b0010) Negative
bit 2 (0b0100) Overflow
bit 3 (0b1000) Carry

```

Set by `ADD`, `ADD_IMM`, `SUB`, `SUB_IMM`, `CMP`, `CMP_IMM` (all four flags
recomputed from scratch each time; no other instruction touches flags).

| Mnemonic | Condition         | Notes            |
| -------- | ----------------- | ---------------- |
| JZ       | Equals            |                  |
| JNZ      | !Equals           |                  |
| JA       | !Carry && !Equals | unsigned "above" |
| JB       | Carry             | identical to JC  |
| JAE      | !Carry            | identical to JNC |
| JBE      | Carry \|\| Equals |                  |
| JN       | Negative          |                  |
| JNN      | !Negative         |                  |
| JO       | Overflow          |                  |
| JNO      | !Overflow         |                  |
| JC       | Carry             | identical to JB  |
| JNC      | !Carry            | identical to JAE |

## 4. Register Format

- `0x00`–`0x0F` → R0–R15 (general-purpose, 2 bytes of storage each)
- `0x10` → RSP (stack pointer, top of stack)

### Function Calling Convention (software convention — not enforced)

- Return register: R0
- Input registers: R1–R4 (5th+ argument passed on the stack)
- General/scratch registers: R5–R9
- Callee-saved ("untouched") registers: R10–R15

Nothing in the VM automatically saves or restores registers around `CALL`/
`RET` beyond the return address itself — "untouched" is a convention the
callee's own code must honor, not something the virtual machine guarantees.

## 5. Syscalls

- Opcode `0xFF`, `Arguments::None` — all parameters come from registers, no
  operand bytes are encoded.
- Syscall number: R1
- `0x01` PRINT: R2 = start address, **R3 = number of memory cells to read**

---

## 6. Virtual Machine

- Registers R0–R15 + RSP: 2 bytes each.
- Pointers/addresses: 2 bytes (16-bit address space).
- **Two separate address spaces** sharing the 0x0000–0xFFFE range but with
  different granularity
  - **Code** (`Vec<u8>`, byte-addressed): where jumps/calls/PC point. Sized
    to the loaded program, not pre-allocated.
  - **Memory** (`Vec<u16>`, word-addressed): where `LOAD*`/`STORE*` and the
    call stack (`PUSH`/`POP`/`CALL`/`RET`) operate. The stack lives in this
    same space, not a separate third region — it grows downward from RSP's
    initial value.

### Initial State

- All registers initialized to `0`, except `RSP = 0xFFFE`.
- PC (`current_instruction`) starts at `0`, i.e. absolute file offset `0x04`
  — for an executable, this is the linker's `JMP` trampoline, so program
  execution effectively starts with an immediate jump to the real entry
  point.
- The runner always exits with R0's value as the process exit code, regardless
  of whether termination was a clean `HALT` or a crash.
  There is no distinct nonzero "crashed" exit status.

## 7. Assembly Source Syntax

- `;` starts a comment; blank lines are ignored.
- A line whose **last** whitespace-separated token ends in `:` defines a
  label at the current program counter:
  - `EXPORT name:` — registers `name` in the object file's export table
    (visible to the linker / other object files).
  - `name:` (without `EXPORT`) — a local label, referenced only within this file.
  - Extra tokens before the label name on either form trigger a "malformed"
    warning but are otherwise ignored (only the last token is used as the
    name).
- A local/foreign reference is written as `.name`. If `name` matches a local
  label in the same file it's resolved immediately and recorded as an
  internal relocation; otherwise it's recorded as an external relocation
  (value `0` placeholder) to be resolved by the linker, which will error if
  no loaded object exports that name.
- Opcode mnemonics are **case-sensitive** and must match the enum spelling
  exactly (`MOV_IMM`, not `mov.imm`, `movi`, etc.).
- Register names are **case-insensitive** (`r1`, `R1`, `Rsp` all accepted).
- Numeric literals: decimal, or `0x`-prefixed hex. No negative-number,
  binary, or character-literal syntax.
- Operands may be separated by spaces and/or commas interchangeably
  (`MOV R1, R2` and `MOV R1 R2` are equivalent).

## 8. Linker Behavior

- Objects are concatenated in the order given on the command line; each
  object's base address = `3` (trampoline size) + total bytecode length of
  all previously-loaded objects.
- Re-exporting an already-seen label name logs a warning but does not abort
  — the later definition wins.
- Missing entry point (`--entry`, default `START`) logs a warning at load
  time but is only a hard error once `link()` actually runs and can't find
  it in the global symbol table.
- An external relocation with no matching export anywhere in the linked set
  is a hard error (`Undefined symbol '...'`).
- Total combined bytecode across all objects must be `≤ 0xFFFF` bytes, and
  the final linked binary (header included) is independently checked
  against the same `0xFFFF` limit.

```

```
