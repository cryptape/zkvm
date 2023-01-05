use ckb_vm::decoder::{build_decoder, Decoder};
use ckb_vm::machine::DefaultMachine;
use ckb_vm::memory::Memory;
use ckb_vm::{Bytes, CoreMachine, Error, Register, SupportMachine};

pub struct TraceRow {
    pub cycles: u32,
    pub pc: u32,
    pub ci: u64, // Current instruction
    pub registers: [u32; 32],
}

pub struct Trace {
    pub cycles: u32,
    pub rows: Vec<TraceRow>,
}

impl Trace {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            rows: Vec::new(),
        }
    }

    fn step_init<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>(
        &mut self,
        machine: &mut DefaultMachine<'a, Inner>,
        decoder: &mut Decoder,
    ) -> Result<(), Error> {
        let pc = machine.pc().clone();
        let inst = decoder.decode(machine.memory_mut(), pc.to_u64())?;
        let mut row_registers = [0u32; 32];
        for i in 0..32 {
            row_registers[i] = machine.registers()[i].to_u32();
        }
        let row = TraceRow {
            cycles: self.cycles,
            pc: pc.to_u32(),
            ci: inst,
            registers: row_registers,
        };
        self.rows.push(row);
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
    pub inner: DefaultMachine<'a, Inner>,
    pub trace: Trace,
}

impl<R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>> CoreMachine
    for PProfMachine<'_, Inner>
{
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
    for PProfMachine<'_, Inner>
{
    fn ecall(&mut self) -> Result<(), Error> {
        self.inner.ecall()
    }

    fn ebreak(&mut self) -> Result<(), Error> {
        self.inner.ebreak()
    }
}

impl<'a, R: Register, M: Memory<REG = R>, Inner: SupportMachine<REG = R, MEM = M>>
    PProfMachine<'a, Inner>
{
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
        Ok(self.inner.exit_code())
    }
}
