use ckb_vm::decoder::{build_decoder, Decoder};
use ckb_vm::machine::DefaultMachine;
use ckb_vm::memory::Memory;
use ckb_vm::{Bytes, CoreMachine, Error, Machine, Register, SupportMachine};

pub struct ProcessorTableRow {
    pub cycles: u32,
    pub registers: [u32; 32],
}

pub struct InstructionTableRow {
    pub pc: u32, // The instruction pointer
    pub ci: u32, // The current instruction
    pub ni: u32, // The next instruction
}

pub struct Tables {
    pub cycles: u32,
    pub processor: Vec<ProcessorTableRow>,
}

impl Tables {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            processor: Vec::new(),
        }
    }

    fn step_init<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<'a, Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        let inst = decoder.decode(machine.memory_mut(), pc)?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);
        let _ = opcode;
        let mut processor_table_row_registers = [0u32; 32];
        for i in 0..32 {
            processor_table_row_registers[i] = machine.registers()[i].to_u32();
        }
        let processor_table_row = ProcessorTableRow {
            cycles: self.cycles,
            registers: processor_table_row_registers,
        };
        self.processor.push(processor_table_row);
        return Ok(());
    }

    fn step_done<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<'a, Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        let pc = machine.pc().to_u64();
        let inst = decoder.decode(machine.memory_mut(), pc)?;
        let opcode = ckb_vm::instructions::extract_opcode(inst);
        let _ = opcode;
        self.cycles += 1;
        return Ok(());
    }
}

pub struct PProfMachine<'a, Inner> {
    pub machine: DefaultMachine<'a, Inner>,
    pub tables: Tables,
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> CoreMachine
    for PProfMachine<'_, Inner>
{
    type REG = <Inner as CoreMachine>::REG;
    type MEM = <Inner as CoreMachine>::MEM;

    fn pc(&self) -> &Self::REG {
        &self.machine.pc()
    }

    fn update_pc(&mut self, pc: Self::REG) {
        self.machine.update_pc(pc)
    }

    fn commit_pc(&mut self) {
        self.machine.commit_pc()
    }

    fn memory(&self) -> &Self::MEM {
        self.machine.memory()
    }

    fn memory_mut(&mut self) -> &mut Self::MEM {
        self.machine.memory_mut()
    }

    fn registers(&self) -> &[Self::REG] {
        self.machine.registers()
    }

    fn set_register(&mut self, idx: usize, value: Self::REG) {
        self.machine.set_register(idx, value)
    }

    fn isa(&self) -> u8 {
        self.machine.isa()
    }

    fn version(&self) -> u32 {
        self.machine.version()
    }
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> Machine
    for PProfMachine<'_, Inner>
{
    fn ecall(&mut self) -> Result<(), Error> {
        self.machine.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.machine.ebreak()
    }
}

impl<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>
    PProfMachine<'a, Inner>
{
    pub fn new(machine: DefaultMachine<'a, Inner>, tables: Tables) -> Self {
        Self { machine, tables }
    }

    pub fn load_program(&mut self, program: &Bytes, args: &[Bytes]) -> Result<u64, Error> {
        self.machine.load_program(program, args)
    }

    pub fn run(&mut self) -> Result<i8, Error> {
        let mut decoder = build_decoder::<Inner::REG>(self.isa(), self.version());
        self.machine.set_running(true);
        while self.machine.running() {
            if self.machine.reset_signal() {
                decoder.reset_instructions_cache();
                self.tables = Tables::new();
            }
            self.tables.step_init(&mut self.machine, &mut decoder)?;
            self.machine.step(&mut decoder)?;
            self.tables.step_done(&mut self.machine, &mut decoder)?;
        }
        Ok(self.machine.exit_code())
    }
}
