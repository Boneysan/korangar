#!/usr/bin/env python3
"""Extract Ragexe's native skill-ID to source actor-state table.

This is deliberately an executable-specific audit tool, not runtime game
logic. It emulates the comparison and jump-table portion of 0x009B4B30 in the
2019-06-05 reference client for every unsigned 16-bit skill ID. The unrelated
Lua GetBeginEffectID result is forced to -1 so the reported value is only the
native actor-state output.

Dependencies:
    python3 -m pip install pefile capstone
"""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

import pefile
from capstone import CS_ARCH_X86, CS_MODE_32, Cs
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG


REFERENCE_SHA256 = "61663a6f3bca42e992e3d61418b9508db57e6ba18cc0069e297eb3730d4d825d"
FUNCTION_START = 0x009B4B30
FUNCTION_END = 0x009B51CE
GET_BEGIN_EFFECT_ID = 0x009AF9A0
MASK = 0xFFFFFFFF


def default_executable() -> Path:
    repository = Path(__file__).resolve().parents[2]
    return repository.parent.parent / "RO" / "client" / "2019-06-05fRagexe_patched.exe"


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=default_executable())
    parser.add_argument(
        "--maximum",
        type=lambda value: int(value, 0),
        default=0xFFFF,
        help="highest skill ID to inspect (default: full u16 domain)",
    )
    return parser.parse_args()


def check_reference(executable: Path) -> None:
    digest = hashlib.sha256(executable.read_bytes()).hexdigest()
    if digest != REFERENCE_SHA256:
        raise SystemExit(
            f"unsupported executable SHA-256 {digest}; expected {REFERENCE_SHA256}. "
            "Recover the function boundaries and instructions before auditing another build."
        )


class SkillStateEmulator:
    def __init__(self, executable: Path) -> None:
        pe = pefile.PE(str(executable), fast_load=True)
        self.base = pe.OPTIONAL_HEADER.ImageBase
        self.image = pe.get_memory_mapped_image()
        self.disassembler = Cs(CS_ARCH_X86, CS_MODE_32)
        self.disassembler.detail = True
        code = self.image[FUNCTION_START - self.base : FUNCTION_END - self.base]
        self.instructions = {instruction.address: instruction for instruction in self.disassembler.disasm(code, FUNCTION_START)}

    def read_image(self, address: int, width: int) -> int:
        offset = address - self.base
        return int.from_bytes(self.image[offset : offset + width], "little")

    def emulate(self, skill_id: int) -> tuple[int | None, int | None]:
        registers = {name: 0 for name in ("eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp")}
        registers["esp"] = 0x30000000
        memory = {
            0x30000008: skill_id,
            0x3000000C: 0x20000010,
            0x30000010: 0x20000000,
        }
        flags = {"zf": False, "cf": False, "sf": False, "of": False}
        pc = FUNCTION_START

        def register_name(register: int) -> str:
            return self.disassembler.reg_name(register)

        def memory_address(native_memory) -> int:
            address = native_memory.disp
            if native_memory.base:
                address += registers[register_name(native_memory.base)]
            if native_memory.index:
                address += registers[register_name(native_memory.index)] * native_memory.scale
            return address & MASK

        def value(operand) -> int:
            if operand.type == X86_OP_IMM:
                return operand.imm & MASK
            if operand.type == X86_OP_REG:
                return registers[register_name(operand.reg)]
            if operand.type == X86_OP_MEM:
                address = memory_address(operand.mem)
                if address in memory:
                    return memory[address]
                return self.read_image(address, operand.size)
            raise RuntimeError(f"unsupported operand at {pc:#x}")

        def write(operand, result: int) -> None:
            result &= MASK
            if operand.type == X86_OP_REG:
                registers[register_name(operand.reg)] = result
            elif operand.type == X86_OP_MEM:
                memory[memory_address(operand.mem)] = result
            else:
                raise RuntimeError(f"unsupported destination at {pc:#x}")

        for _ in range(1000):
            instruction = self.instructions.get(pc)
            if instruction is None:
                raise RuntimeError(f"no instruction for skill {skill_id} at {pc:#x}")
            next_pc = pc + instruction.size
            mnemonic = instruction.mnemonic
            operands = instruction.operands

            if mnemonic in ("push", "pop"):
                pass
            elif mnemonic in ("mov", "movzx"):
                write(operands[0], value(operands[1]))
            elif mnemonic == "lea":
                write(operands[0], memory_address(operands[1].mem))
            elif mnemonic in ("add", "sub"):
                left = value(operands[0])
                right = value(operands[1])
                write(operands[0], left + right if mnemonic == "add" else left - right)
            elif mnemonic == "cmp":
                left = value(operands[0])
                right = value(operands[1])
                result = (left - right) & MASK
                flags["zf"] = result == 0
                flags["cf"] = left < right
                flags["sf"] = bool(result & 0x80000000)
                flags["of"] = bool((left ^ right) & (left ^ result) & 0x80000000)
            elif mnemonic == "call":
                target = value(operands[0])
                if target != GET_BEGIN_EFFECT_ID:
                    raise RuntimeError(f"unexpected call {target:#x} at {pc:#x}")
                registers["eax"] = MASK
            elif mnemonic == "jmp":
                next_pc = value(operands[0])
            elif mnemonic in ("je", "jne", "ja", "jg", "jge"):
                take = {
                    "je": flags["zf"],
                    "jne": not flags["zf"],
                    "ja": not flags["cf"] and not flags["zf"],
                    "jg": not flags["zf"] and flags["sf"] == flags["of"],
                    "jge": flags["sf"] == flags["of"],
                }[mnemonic]
                if take:
                    next_pc = value(operands[0])
            elif mnemonic == "ret":
                return memory.get(0x20000010), memory.get(0x20000000)
            else:
                raise RuntimeError(f"unsupported {mnemonic} at {pc:#x}")

            pc = next_pc

        raise RuntimeError(f"instruction limit for skill {skill_id} at {pc:#x}")


def contiguous_ranges(values: list[int]):
    start = previous = values[0]
    for current in values[1:]:
        if current != previous + 1:
            yield start, previous
            start = current
        previous = current
    yield start, previous


def main() -> None:
    arguments = parse_arguments()
    if not 0 <= arguments.maximum <= 0xFFFF:
        raise SystemExit("--maximum must be in the unsigned 16-bit domain")
    check_reference(arguments.executable)
    emulator = SkillStateEmulator(arguments.executable)
    groups: dict[int | None, list[int]] = {}
    for skill_id in range(arguments.maximum + 1):
        _, actor_state = emulator.emulate(skill_id)
        groups.setdefault(actor_state, []).append(skill_id)

    key = lambda item: -1 if item[0] is None else item[0]
    for actor_state, skill_ids in sorted(groups.items(), key=key):
        formatted = ", ".join(
            str(first) if first == last else f"{first}-{last}"
            for first, last in contiguous_ranges(skill_ids)
        )
        state_label = "-1 (0xffffffff)" if actor_state == MASK else str(actor_state)
        print(f"state {state_label}: {formatted}")


if __name__ == "__main__":
    main()
