use halo2_proofs::{dev::MockProver, pasta::Fp};
use std::marker::PhantomData;

fn test_version_1() {
    use fibonacci::version_1::FibonacciCircuit;

    println!("Hello, this is halo2 example: fabonacci_1...");
    // 定义电路的行数
    let row = 4;

    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);

    // 用隐私输入实例化电路，这里没有隐私输入，所以输入占位符
    let circuit: FibonacciCircuit<Fp> = FibonacciCircuit(PhantomData);

    // 输入正确的 public input ,验证成功
    let public_input = vec![a, b, out];
    let prover = MockProver::run(row, &circuit, vec![public_input]).unwrap();
    // println!("res1: {:?}", prover);
    let res = prover.verify();
    println!("res1: {:?}", res);

    // 输入错误的 public input ,验证错误
    let out_2 = Fp::from(56);
    let public_input_2 = vec![a, b, out_2];
    let prover_2 = MockProver::run(row, &circuit, vec![public_input_2]).unwrap();
    let res_2 = prover_2.verify();
    println!("res2: {:?}", res_2);
}

fn test_version_2() {
    use fibonacci::version_2::FibonacciCircuit;

    println!("Hello, this is halo2 example: fabonacci_2...");
    // 定义电路的行数
    let row = 4;

    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);

    // 用隐私输入实例化电路，这里没有隐私输入，所以输入占位符
    let circuit: FibonacciCircuit<Fp> = FibonacciCircuit(PhantomData);

    // 输入正确的 public input ,验证成功
    let public_input = vec![a, b, out];
    let prover = MockProver::run(row, &circuit, vec![public_input]).unwrap();
    // println!("res1: {:?}", prover);
    let res = prover.verify();
    println!("res1: {:?}", res);

    // 输入错误的 public input ,验证错误
    let out_2 = Fp::from(56);
    let public_input_2 = vec![a, b, out_2];
    let prover_2 = MockProver::run(row, &circuit, vec![public_input_2]).unwrap();
    let res_2 = prover_2.verify();
    println!("res2: {:?}", res_2);
}
fn main() {
    test_version_1();
    println!("-------------------------");
    test_version_2();
}
