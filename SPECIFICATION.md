# VMR Specification

## File Format

0x00 - 0x03 Magic Number: 0x76 0x6d 0x72
0x04: 0x87 For Executable 0x6F For Object File
x03 ... Code
After the code, any data.

## Instruction Format

OPCODE is 1 byte

Pipe (|) means boundary. Immediate is always 2 bytes.

```
1 byte OPCODE
         then nothing
         or REG
         or REG       | REG 
         or REG       | Immediate
         or Immediate | REG
         or REG       | Immediate | REG
```

### Argument Definition

IMM is always 2 bytes 

```
- 0x0 -> No argument, next byte start of instruction
- 0x1 -> REG
- 0x2 -> REG | REG
- 0x3 -> REG | IMM
- 0x4 -> IMM | REG
```

### Register Format

Addressed by one byte.

- 0x00 to 0x0F -> 2 byte registers
- 0x10 -> RSP (Points to the top of the stack)

### Function Specification

- Return register: R0

- Input Registers: R1, R2, R3, R4. For > 5, the stack is used

- General Registers: R5, R6, R6, R8, R9.

- Untouched Registers: R10, R11, R12, R13, R14, R15.

### Syscall

- OPCODE: 0xFF

- SYSCALL NUMBER: R1

- R1 0x01 PRINT. R2 Address. R3 Number of bytes (1 byte = 16 bit)

## Virtual Machine

Registers R0 - R15 contain 2 bytes each. 

Pointers are 2 bytes

Two memory strips:

0x0000 - 0xFFFF Code (when addressed by jumps)

0x0000 - 0xFFFF Memory (when addressed by load, store, etc. ) 
