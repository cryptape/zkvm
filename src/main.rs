fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flag_parser = clap::App::new("zkvm")
        .version("0.2.1")
        .arg(
            clap::Arg::with_name("bin")
                .long("bin")
                .value_name("filename")
                .help("Specify the name of the executable")
                .required(true),
        )
        .arg(
            clap::Arg::with_name("arg")
                .long("arg")
                .value_name("arguments")
                .help("Pass arguments to binary")
                .multiple(true),
        )
        .get_matches();
    let fl_bin = flag_parser.value_of("bin").unwrap();
    let fl_arg: Vec<_> = flag_parser.values_of("arg").unwrap_or_default().collect();

    let code_data = std::fs::read(fl_bin)?;
    let code = ckb_vm::Bytes::from(code_data);
    let isa = ckb_vm::ISA_IMC;
    let default_core_machine = ckb_vm::DefaultCoreMachine::<u32, ckb_vm::memory::flat::FlatMemory<u32>>::new(
        isa,
        ckb_vm::machine::VERSION1,
        1 << 32,
    );
    let default_machine =
        ckb_vm::DefaultMachineBuilder::new(default_core_machine).instruction_cycle_func(&|_| 0).build();
    let tables = zkvm::Trace::new();
    let mut machine = zkvm::ZkMachine::new(default_machine, tables);
    let mut args = vec![];
    args.append(&mut fl_arg.iter().map(|x| ckb_vm::Bytes::from(x.to_string())).collect());
    machine.load_program(&code, &args)?;
    let exit = machine.run();
    println!("{:?}", exit);
    println!("processor_table_rows={:?}", machine.trace.processor.len());
    println!("instruction_table_rows={:?}", machine.trace.instruction.len());
    println!("memory_table_rows={:?}", machine.trace.memory.len());
    Ok(())
}
