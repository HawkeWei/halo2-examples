#![allow(unused)]
use std::marker::PhantomData;

use group::ff::Field;
use halo2_proofs::{
    circuit::{layouter, AssignedCell, Chip, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};

///////////////////////////////////////////////////////////////////////
/// 重写 fibonacci：
/// 删除每次从上一行赋值pre_b和pre_c，改用直接访问多行：只用1列表示，当前值等于上一行+上上一行
///

///////////////////////////////////////////////////////////////////////
/// 本例中不需要自定义的指令，所以这里直接创建自定义芯片和芯片的配置结构
///
#[derive(Clone, Debug)]
pub struct FibonacciConfig {
    advice: Column<Advice>,
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
pub struct ACell<F: Field>(AssignedCell<F, F>);

impl<F: Field> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        FibonacciChip {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
        instance: Column<Instance>,
    ) -> FibonacciConfig {
        meta.enable_equality(advice);
        meta.enable_equality(instance);

        let selector = meta.selector();

        meta.create_gate("add", |meta| {
            // | a0  | selector
            // | a   | s
            // | b   | s
            // | c   | s
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            let s = meta.query_selector(selector);
            vec![(s * (a + b - c))]
        });

        FibonacciConfig {
            advice,
            instance,
            selector,
        }
    }

    pub fn assign_row(&self, mut layouter: impl Layouter<F>, n: usize) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "first row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                let mut a = region
                    .assign_advice_from_instance(
                        || "f(0)",
                        self.config.instance,
                        0,
                        self.config.advice,
                        0,
                    )
                    .map(ACell)?;
                let mut b = region
                    .assign_advice_from_instance(
                        || "f(1)",
                        self.config.instance,
                        1,
                        self.config.advice,
                        1, // 复制到当前的 region 的 row 1
                    )
                    .map(ACell)?;

                for row in 2..n {
                    if row < n - 2 {
                        self.config.selector.enable(&mut region, row)?;
                    }

                    let mut c = region
                        .assign_advice(
                            || "f(n)",
                            self.config.advice,
                            row,
                            || a.0.value().copied() + b.0.value(),
                        )
                        .map(ACell)?;
                    a = b;
                    b = c;
                }
                Ok((b))
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        c: ACell<F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(c.0.cell(), self.config.instance, row)
    }
}

///////////////////////////////////////////////////////////////////////
/// 使用上面自定义的芯片来构建电路
///
#[derive(Default)]
pub struct FibonacciCircuit<F>(pub PhantomData<F>);

impl<F: Field> Circuit<F> for FibonacciCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();
        FibonacciChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let fibonacci_chip = FibonacciChip::construct(config);

        let out = fibonacci_chip.assign_row(layouter.namespace(|| "entire table"), 10)?;

        fibonacci_chip.expose_public(layouter.namespace(|| "out"), out, 2);

        Ok(())
    }
}
