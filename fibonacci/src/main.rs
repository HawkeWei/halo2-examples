#![allow(unused)]
use group::ff::Field;
use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    dev::MockProver,
    pasta::Fp,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

///////////////////////////////////////////////////////////////////////
/// 本例中不需要自定义的指令，所以这里直接创建自定义芯片和芯片的配置结构
///
#[derive(Debug, Clone)]
struct FibonacciConfig {
    advice: [Column<Advice>; 3],
    instance: Column<Instance>,
    selector: Selector,
}

#[derive(Debug, Clone)]
struct FibonacciChip<F: Field> {
    config: FibonacciConfig,
    _marker: PhantomData<F>,
}

///////////////////////////////////////////////////////////////////////
/// 实现自定义芯片：包括实现芯片配置，和芯片中的其他功能
///

/// 这里先定义 tuple struct ACell，用于简化与电路中单元格的交互（原因见simple example）
#[derive(Debug, Clone)]
struct ACell<F: Field>(AssignedCell<F, F>);

impl<F: Field> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// 实现配置，构建约束，创建 custom gate：s * (a0 + a1 - a2) == 0
    /// Fibonacci 数列的特性： (row_i, a0) = (row_i-1, a1), (row_i, a1) = (row_i-1, a2);
    fn configure(
        meta: &mut ConstraintSystem<F>, // 对约束系统的可变引用，配置column、custom gate、对应的约束
        advice: [Column<Advice>; 3],    // 选择器
        instance: Column<Instance>,
    ) -> FibonacciConfig {
        meta.enable_equality(instance);
        // 启用强制执行指定列中的单元格相等的功能，否则报错：Err(ColumnNotInPermutation(Column { index: 0, column_type: Advice }))
        meta.enable_equality(advice[0]);
        meta.enable_equality(advice[1]);
        meta.enable_equality(advice[2]);

        let selector = meta.selector();

        meta.create_gate("add", |meta| {
            // | a0  | a1  | a2 | selector
            // | a   | b   | c  | s
            let a = meta.query_advice(advice[0], Rotation::cur());
            let b = meta.query_advice(advice[1], Rotation::cur());
            let c = meta.query_advice(advice[2], Rotation::cur());
            let s = meta.query_selector(selector);
            vec![s * (a + b - c)]
        });

        FibonacciConfig {
            advice,
            instance,
            selector,
        }
    }
    ///////////////////////////////////////////////////////////////////////
    /// 实现芯片的核心功能：
    /// 1、初始化第一行（1，1，2）
    /// 2、根据 Fibonacci 数列的特性，进行循环赋值和计算
    /// 3、expose public
    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        // let mut config = self.config();
        layouter.assign_region(
            || "first row",
            |mut region| {
                // 激活加法
                self.config.selector.enable(&mut region, 0);
                // f(0) = 1
                let a = region
                    .assign_advice_from_instance(
                        || "f(0)",
                        self.config.instance,
                        0,
                        self.config.advice[0],
                        0,
                    )
                    .map(ACell)?;
                // f(1) = 1
                let b = region
                    .assign_advice_from_instance(
                        || "f(1)",
                        self.config.instance,
                        0,
                        self.config.advice[1],
                        0,
                    )
                    .map(ACell)?;
                // f(2) = f(0) + f(1)
                let c = region
                    .assign_advice(
                        || "f(2)",
                        self.config.advice[2],
                        0,
                        || a.0.value().copied() + b.0.value(),
                    )
                    .map(ACell)?;
                Ok((a, b, c))
            },
        )
    }

    fn assign_row(
        &self,
        mut layouter: impl Layouter<F>,
        pre_b: &ACell<F>,
        pre_c: &ACell<F>,
    ) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "next row",
            |mut region| {
                self.config.selector.enable(&mut region, 0);
                // 拷贝约束，本次的a = 前一次的b，本次的b = 前一次的c
                pre_b
                    .0
                    .copy_advice(|| "a", &mut region, self.config.advice[0], 0)?;
                pre_c
                    .0
                    .copy_advice(|| "b", &mut region, self.config.advice[1], 0)?;
                // 计算本次的c = a + b = pre_b + pre_c
                let c = region
                    .assign_advice(
                        || "c",
                        self.config.advice[2],
                        0,
                        || pre_b.0.value().copied() + pre_c.0.value(),
                    )
                    .map(ACell)?;
                Ok((c))
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        c: &ACell<F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(c.0.cell(), self.config.instance, row)
    }
}

///////////////////////////////////////////////////////////////////////
/// 使用上面自定义的芯片来构建电路
///

/// 电路中没有私有输入，所以这里定义电路结构体时，仅使用占位符
#[derive(Default)]
struct FibonacciCircuit<F>(PhantomData<F>);

impl<F: Field> Circuit<F> for FibonacciCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;

    /// 返回此电路的副本，没有 witness（即所有witness设置为 None）。对于大多数电路，这将等于Self::default()。
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    /// 输入约束系统，输出之前自定义的 simpleConfig
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];
        let instance = meta.instance_column();

        FibonacciChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let fibonacci_chip = FibonacciChip::construct(config);

        let (_, mut pre_b, mut pre_c) =
            fibonacci_chip.assign_first_row(layouter.namespace(|| "first row"))?;

        for _i in 3..10 {
            let c = fibonacci_chip.assign_row(layouter.namespace(|| "next row"), &pre_b, &pre_c)?;
            pre_b = pre_c;
            pre_c = c;
        }

        fibonacci_chip.expose_public(layouter.namespace(|| "out"), &pre_c, 2)?;

        Ok(())
    }
}

fn main() {
    println!("Hello, this is halo2 example: fabonacci...");
    // 定义电路的行数
    let row = 4;

    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);

    // 用隐私输入实例化电路，这里没有隐私输入，所以输入占位符
    let circuit: FibonacciCircuit<Fp> = FibonacciCircuit(PhantomData);

    let mut public_input = vec![a, b, out];

    let prover = MockProver::run(row, &circuit, vec![public_input]).unwrap();
    // println!("res1: {:?}", prover);
    let res1 = prover.verify();
    println!("res1: {:?}", res1);
}
