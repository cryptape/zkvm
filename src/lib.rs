use ckb_vm::decoder::{build_decoder, Decoder};
use ckb_vm::machine::DefaultMachine;
use ckb_vm::memory::Memory;
use ckb_vm::{Bytes, CoreMachine, Error, Register, SupportMachine};

#[derive(Debug, Default)]
pub struct ProcessorRow {
    pub cycles: u32,
    pub pc: u32,
    // Current instruction.
    pub ci: u64,
    // Next instruction. Program is done when ni == OP_UNLOADED.
    pub ni: u64,
    pub registers: [u32; 32],
}

#[derive(Debug, Default)]
pub struct InstructionRow {
    pub pc: u32,
    pub ci: u64,
    pub ni: u64,
}

#[derive(Debug)]
pub enum MemoryOperation {
    R,
    W,
}

#[derive(Debug)]
pub struct MemoryRow {
    pub addr: u32,
    pub cycles: u32,
    pub value: u8,
    pub op: MemoryOperation,
}

pub struct Trace {
    pub cycles: u32,
    pub processor: Vec<ProcessorRow>,
    pub instruction: Vec<InstructionRow>,
    pub memory: Vec<MemoryRow>,
}

impl Trace {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            processor: Vec::new(),
            instruction: Vec::new(),
            memory: Vec::new(),
        }
    }

    fn step_init<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<'a, Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        let pc = machine.pc().clone();
        let inst = decoder.decode(machine.memory_mut(), pc.to_u64())?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);

        let mut processor_row = ProcessorRow::default();
        processor_row.cycles = self.cycles;
        processor_row.pc = pc.to_u32();
        processor_row.ci = inst;
        processor_row.ni = 0;
        for i in 0..32 {
            processor_row.registers[i] = machine.registers()[i].to_u32();
        }
        self.processor.push(processor_row);

        let instruction_row = InstructionRow {
            pc: pc.to_u32(),
            ci: inst,
            ni: 0,
        };
        self.instruction.push(instruction_row);

        match opcode {
            ckb_vm_definitions::instructions::OP_LB | ckb_vm_definitions::instructions::OP_LBU => {
                let i = ckb_vm::instructions::Itype(inst);
                let rs1 = i.rs1();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                for _ in 0..1 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: machine.memory_mut().load8(&addr)?.to_u8(),
                        op: MemoryOperation::R,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_LH | ckb_vm_definitions::instructions::OP_LHU => {
                let i = ckb_vm::instructions::Itype(inst);
                let rs1 = i.rs1();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                for _ in 0..2 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: machine.memory_mut().load8(&addr)?.to_u8(),
                        op: MemoryOperation::R,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_LW | ckb_vm_definitions::instructions::OP_LWU => {
                let i = ckb_vm::instructions::Itype(inst);
                let rs1 = i.rs1();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                for _ in 0..4 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: machine.memory_mut().load8(&addr)?.to_u8(),
                        op: MemoryOperation::R,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_LD => {
                let i = ckb_vm::instructions::Itype(inst);
                let rs1 = i.rs1();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                for _ in 0..8 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: machine.memory_mut().load8(&addr)?.to_u8(),
                        op: MemoryOperation::R,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_SB => {
                let i = ckb_vm::instructions::Stype(inst);
                let rs1 = i.rs1();
                let rs2 = i.rs2();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                let value = machine.registers()[rs2 as usize].to_u64();
                for i in 0..1 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: (value >> (i * 8)) as u8,
                        op: MemoryOperation::W,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_SH => {
                let i = ckb_vm::instructions::Stype(inst);
                let rs1 = i.rs1();
                let rs2 = i.rs2();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                let value = machine.registers()[rs2 as usize].to_u64();
                for i in 0..2 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: (value >> (i * 8)) as u8,
                        op: MemoryOperation::W,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_SW => {
                let i = ckb_vm::instructions::Stype(inst);
                let rs1 = i.rs1();
                let rs2 = i.rs2();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                let value = machine.registers()[rs2 as usize].to_u64();
                for i in 0..4 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: (value >> (i * 8)) as u8,
                        op: MemoryOperation::W,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            ckb_vm_definitions::instructions::OP_SD => {
                let i = ckb_vm::instructions::Stype(inst);
                let rs1 = i.rs1();
                let rs2 = i.rs2();
                let imm = i.immediate_s();
                let mut addr = machine.registers()[rs1 as usize].overflowing_add(&R::from_i32(imm));
                let value = machine.registers()[rs2 as usize].to_u64();
                for i in 0..8 {
                    self.memory.push(MemoryRow {
                        addr: addr.to_u32(),
                        cycles: self.cycles,
                        value: (value >> (i * 8)) as u8,
                        op: MemoryOperation::W,
                    });
                    addr = addr.overflowing_add(&R::from_i32(1));
                }
            }
            _ => {}
        }

        return Ok(());
    }

    fn step_done<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<'a, Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        if machine.running() {
            let pc = machine.pc().to_u64();
            let inst = decoder.decode(machine.memory_mut(), pc)?;
            self.processor.last_mut().unwrap().ni = inst;
            self.instruction.last_mut().unwrap().ni = inst;
        } else {
            let inst = ckb_vm::instructions::blank_instruction(ckb_vm_definitions::instructions::OP_UNLOADED);
            self.processor.last_mut().unwrap().ni = inst;
            self.instruction.last_mut().unwrap().ni = inst;
        }
        self.cycles += 1;
        return Ok(());
    }

    fn done(&mut self) {
        self.instruction.sort_by(|a, b| a.pc.cmp(&b.pc));
        self.memory.sort_by(|a, b| {
            if a.addr != b.addr {
                a.addr.cmp(&b.addr)
            } else {
                a.cycles.cmp(&b.cycles)
            }
        });
    }
}

pub struct ZkMachine<'a, Inner> {
    pub inner: DefaultMachine<'a, Inner>,
    pub trace: Trace,
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> CoreMachine for ZkMachine<'_, Inner> {
    type REG = <Inner as CoreMachine>::REG;
    type MEM = <Inner as CoreMachine>::MEM;

    fn pc(&self) -> &Self::REG {
        &self.inner.pc()
    }

    fn update_pc(&mut self, pc: Self::REG) {
        self.inner.update_pc(pc)
    }

    fn commit_pc(&mut self) {
        self.inner.commit_pc()
    }

    fn memory(&self) -> &Self::MEM {
        self.inner.memory()
    }

    fn memory_mut(&mut self) -> &mut Self::MEM {
        self.inner.memory_mut()
    }

    fn registers(&self) -> &[Self::REG] {
        self.inner.registers()
    }

    fn set_register(&mut self, idx: usize, value: Self::REG) {
        self.inner.set_register(idx, value)
    }

    fn isa(&self) -> u8 {
        self.inner.isa()
    }

    fn version(&self) -> u32 {
        self.inner.version()
    }
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> ckb_vm::Machine
    for ZkMachine<'_, Inner>
{
    fn ecall(&mut self) -> Result<(), Error> {
        self.inner.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.inner.ebreak()
    }
}

impl<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> ZkMachine<'a, Inner> {
    pub fn new(inner: DefaultMachine<'a, Inner>, trace: Trace) -> Self {
        Self {
            inner: inner,
            trace: trace,
        }
    }

    pub fn load_program(&mut self, program: &Bytes, args: &[Bytes]) -> Result<u64, Error> {
        self.inner.load_program(program, args)
    }

    pub fn run(&mut self) -> Result<i8, Error> {
        let mut decoder = build_decoder::<Inner::REG>(self.isa(), self.version());
        self.inner.set_running(true);
        while self.inner.running() {
            if self.inner.reset_signal() {
                decoder.reset_instructions_cache();
                self.trace = Trace::new();
            }
            self.trace.step_init(&mut self.inner, &mut decoder)?;
            self.inner.step(&mut decoder)?;
            self.trace.step_done(&mut self.inner, &mut decoder)?;
        }
        self.trace.done();
        Ok(self.inner.exit_code())
    }
}
