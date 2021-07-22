/*
 * Copyright (C) 2021 The Aero Project Developers.
 *
 * This file is part of The Aero Project.
 *
 * Aero is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Aero is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Aero. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::mem::paging::{PhysAddr, PhysFrame, VirtAddr};

bitflags::bitflags! {
    /// Controls cache settings for the level 4 page table.
    pub struct Cr3Flags: u64 {
        /// Use a writethrough cache policy for the P4 table (else a writeback policy is used).
        const PAGE_LEVEL_WRITETHROUGH = 1 << 3;
        /// Disable caching for the P4 table.
        const PAGE_LEVEL_CACHE_DISABLE = 1 << 4;
    }
}

bitflags::bitflags! {
    /// Controls cache settings for the level 4 page table.
    pub struct Cr4Flags: u64 {
        /// Enables hardware-supported performance enhancements for software running in
        /// virtual-8086 mode.
        const VIRTUAL_8086_MODE_EXTENSIONS = 1;
        /// Enables support for protected-mode virtual interrupts.
        const PROTECTED_MODE_VIRTUAL_INTERRUPTS = 1 << 1;
        /// When set, only privilege-level 0 can execute the RDTSC or RDTSCP instructions.
        const TIMESTAMP_DISABLE = 1 << 2;
        /// Enables I/O breakpoint capability and enforces treatment of DR4 and DR5 registers
        /// as reserved.
        const DEBUGGING_EXTENSIONS = 1 << 3;
        /// Enables the use of 4MB physical frames; ignored in long mode.
        const PAGE_SIZE_EXTENSION = 1 << 4;
        /// Enables physical address extension and 2MB physical frames; required in long mode.
        const PHYSICAL_ADDRESS_EXTENSION = 1 << 5;
        /// Enables the machine-check exception mechanism.
        const MACHINE_CHECK_EXCEPTION = 1 << 6;
        /// Enables the global-page mechanism, which allows to make page translations global
        /// to all processes.
        const PAGE_GLOBAL = 1 << 7;
        /// Allows software running at any privilege level to use the RDPMC instruction.
        const PERFORMANCE_MONITOR_COUNTER = 1 << 8;
        /// Enable the use of legacy SSE instructions; allows using FXSAVE/FXRSTOR for saving
        /// processor state of 128-bit media instructions.
        const OSFXSR = 1 << 9;
        /// Enables the SIMD floating-point exception (#XF) for handling unmasked 256-bit and
        /// 128-bit media floating-point errors.
        const OSXMMEXCPT_ENABLE = 1 << 10;
        /// Prevents the execution of the SGDT, SIDT, SLDT, SMSW, and STR instructions by
        /// user-mode software.
        const USER_MODE_INSTRUCTION_PREVENTION = 1 << 11;
        /// Enables 5-level paging on supported CPUs.
        const L5_PAGING = 1 << 12;
        /// Enables VMX insturctions.
        const VIRTUAL_MACHINE_EXTENSIONS = 1 << 13;
        /// Enables SMX instructions.
        const SAFER_MODE_EXTENSIONS = 1 << 14;
        /// Enables software running in 64-bit mode at any privilege level to read and write
        /// the FS.base and GS.base hidden segment register state.
        const FSGSBASE = 1 << 16;
        /// Enables process-context identifiers (PCIDs).
        const PCID = 1 << 17;
        /// Enables extendet processor state management instructions, including XGETBV and XSAVE.
        const OSXSAVE = 1 << 18;
        /// Prevents the execution of instructions that reside in pages accessible by user-mode
        /// software when the processor is in supervisor-mode.
        const SUPERVISOR_MODE_EXECUTION_PROTECTION = 1 << 20;
        /// Enables restrictions for supervisor-mode software when reading data from user-mode
        /// pages.
        const SUPERVISOR_MODE_ACCESS_PREVENTION = 1 << 21;
        /// Enables 4-level paging to associate each linear address with a protection key.
        const PROTECTION_KEY = 1 << 22;
    }
}

bitflags::bitflags! {
    /// The RFLAGS register.
    pub struct RFlags: u64 {
        /// Processor feature identification flag.
        ///
        /// If this flag is modifiable, the CPU supports CPUID.
        const ID = 1 << 21;
        /// Indicates that an external, maskable interrupt is pending.
        ///
        /// Used when virtual-8086 mode extensions (CR4.VME) or protected-mode virtual
        /// interrupts (CR4.PVI) are activated.
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
        /// Virtual image of the INTERRUPT_FLAG bit.
        ///
        /// Used when virtual-8086 mode extensions (CR4.VME) or protected-mode virtual
        /// interrupts (CR4.PVI) are activated.
        const VIRTUAL_INTERRUPT = 1 << 19;
        /// Enable automatic alignment checking if CR0.AM is set. Only works if CPL is 3.
        const ALIGNMENT_CHECK = 1 << 18;
        /// Enable the virtual-8086 mode.
        const VIRTUAL_8086_MODE = 1 << 17;
        /// Allows to restart an instruction following an instrucion breakpoint.
        const RESUME_FLAG = 1 << 16;
        /// Used by `iret` in hardware task switch mode to determine if current task is nested.
        const NESTED_TASK = 1 << 14;
        /// The high bit of the I/O Privilege Level field.
        ///
        /// Specifies the privilege level required for executing I/O address-space instructions.
        const IOPL_HIGH = 1 << 13;
        /// The low bit of the I/O Privilege Level field.
        ///
        /// Specifies the privilege level required for executing I/O address-space instructions.
        const IOPL_LOW = 1 << 12;
        /// Set by hardware to indicate that the sign bit of the result of the last signed integer
        /// operation differs from the source operands.
        const OVERFLOW_FLAG = 1 << 11;
        /// Determines the order in which strings are processed.
        const DIRECTION_FLAG = 1 << 10;
        /// Enable interrupts.
        const INTERRUPT_FLAG = 1 << 9;
        /// Enable single-step mode for debugging.
        const TRAP_FLAG = 1 << 8;
        /// Set by hardware if last arithmetic operation resulted in a negative value.
        const SIGN_FLAG = 1 << 7;
        /// Set by hardware if last arithmetic operation resulted in a zero value.
        const ZERO_FLAG = 1 << 6;
        /// Set by hardware if last arithmetic operation generated a carry ouf of bit 3 of the
        /// result.
        const AUXILIARY_CARRY_FLAG = 1 << 4;
        /// Set by hardware if last result has an even number of 1 bits (only for some operations).
        const PARITY_FLAG = 1 << 2;
        /// Set by hardware if last arithmetic operation generated a carry out of the
        /// most-significant bit of the result.
        const CARRY_FLAG = 1;
    }
}

/// Returns the current value of the RFLAGS register.
pub fn read_rflags() -> RFlags {
    let value: u64;

    unsafe {
        asm!("pushf; pop {}", out(reg) value);
    }

    RFlags::from_bits_truncate(value)
}

/// Read the current set of CR4 flags.
#[inline]
pub fn read_cr4() -> Cr4Flags {
    let value: u64;

    unsafe {
        asm!("mov {}, cr4", out(reg) value, options(nostack));
    }

    Cr4Flags::from_bits_truncate(value) // Get the flags from the bits.
}

/// Read the current P4 table address from the CR3 register.
#[inline]
pub fn read_cr3() -> (PhysFrame, Cr3Flags) {
    let value: u64;

    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem));
    }

    let addr = PhysAddr::new(value & 0x_000f_ffff_ffff_f000); // Grab the frame address
    let frame = PhysFrame::containing_address(addr); // Get the frame containing the address

    let flags = Cr3Flags::from_bits_truncate(value & 0xFFF); // Get the flags

    (frame, flags)
}

/// Read the current page fault linear address from the CR2 register.
#[inline]
pub fn read_cr2() -> VirtAddr {
    let value: u64;

    unsafe {
        asm!("mov {}, cr2", out(reg) value, options(nomem));

        // Do not perform address checks on the address stored in the
        // CR2 control register as we want the address to be accurate and
        // not tranucated in the [`VirtAddr::new`] function.
        VirtAddr::new_unsafe(value)
    }
}
