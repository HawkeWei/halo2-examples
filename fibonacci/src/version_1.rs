// #![allow(unused)]
use group::ff::Field;
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

///////////////////////////////////////////////////////////////////////
/// 本例中不需要自定义的指令，所以这里直接创建自定义芯片和芯片的配置结构
///
#[derive(Debug, Clone)]
pub struct FibonacciConfig {
    advice: [Column<Advice>; 3],
    instance: Column<Instance>,
    selector: Selector,
}

#[derive(Debug, Clone)]
pub struct FibonacciChip<F: Field> {
    config: FibonacciConfig,
    _marker: PhantomData<F>,
}

///////////////////////////////////////////////////////////////////////
/// 实现自定义芯片：包括实现芯片配置，和芯片中的其他功能
///

/// 这里先定义 tuple struct ACell，用于简化与电路中单元格的交互（原因见simple example）
#[derive(Debug, Clone)]
pub struct ACell<F: Field>(AssignedCell<F, F>);

impl<F: Field> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// 实现配置，构建约束，创建 custom gate：s * (a0 + a1 - a2) == 0
    /// Fibonacci 数列的特性： (row_i, a0) = (row_i-1, a1), (row_i, a1) = (row_i-1, a2);
    pub fn configure(
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
    /// 1、初始化第一行为固定值（1，1，2）
    /// 2、根据 Fibonacci 数列的特性，进行循环赋值和计算
    /// 3、expose public

    pub fn assign_row(&self, mut layouter: impl Layouter<F>, n: usize) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "next row",
            |mut region| {
                // ?将错误return，消除unused的警告
                self.config.selector.enable(&mut region, 0)?;
                // 拷贝约束，本次的a = 前一次的b，本次的b = 前一次的c
                let mut a = region
                    .assign_advice_from_instance(
                        || "f(0)",
                        self.config.instance,
                        0,
                        self.config.advice[0],
                        0,
                    )
                    .map(ACell)?;
                // f(1) = 1, 从 instance(public input)中获取
                let mut b = region
                    .assign_advice_from_instance(
                        || "f(1)",
                        self.config.instance,
                        0,
                        self.config.advice[1],
                        0,
                    )
                    .map(ACell)?;

                let mut c = region
                    .assign_advice(
                        || "f(2)",
                        self.config.advice[2],
                        0,
                        || a.0.value().copied() + b.0.value().copied(),
                    )
                    .map(ACell)?;
                if n == 0 {
                    Ok(a)
                } else if n == 1 {
                    Ok(b)
                } else {
                    for row in 1..n - 2 {
                        a =
                            b.0.copy_advice(|| "a", &mut region, self.config.advice[0], row)
                                .map(ACell)?;
                        b =
                            c.0.copy_advice(|| "b", &mut region, self.config.advice[1], row)
                                .map(ACell)?;
                        // 计算本次的c = a + b = pre_b + pre_c
                        c = region
                            .assign_advice(
                                || "f(n)",
                                self.config.advice[2],
                                row,
                                || a.0.value().copied() + b.0.value().copied(),
                            )
                            .map(ACell)?;
                    }
                    Ok(c)
                }
            },
        )
    }

    pub fn expose_public(
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
pub struct FibonacciCircuit<F>(pub PhantomData<F>);

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

        let c = fibonacci_chip.assign_row(layouter.namespace(|| "next row"), 10)?;

        fibonacci_chip.expose_public(layouter.namespace(|| "out"), &c, 2)?;

        Ok(())
    }
}
