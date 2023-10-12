#![allow(unused)]
use group::ff::Field;
use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Column, Error, Instance, Selector},
};
use std::marker::PhantomData;
/// 这是学习 halo2 的第一个应用例子，主要用来熟悉 zcash-halo2 所提供的API。
/// 例子用来计算和证明 a^2 * b^2 = c, 其中 a、b 为 private input，c 为 public input
///

///////////////////////////////////////////////////////////////////////
/// 定义自定义的指令集，本例中指令包括4个指令：加载私有变量， 加载常量， 计算2个数的乘法，导出公共输入
///
/// 定义一个 NumInstructions trait，要求实现这个 trait 的类型，需要先实现在F域上 Chip 的 trait.
trait NumInstructions<F: Field>: Chip<F> {
    /// 要求实现这个trait时，先确定一个Num的类型
    type Num;
    /// 指令1：加载私有变量. 等价于 load_private<L: Layouter<F>>(&self, layouter: L, a: Value<F>)
    fn load_private(&self, layouter: impl Layouter<F>, a: Value<F>) -> Result<Self::Num, Error>;
    /// 指令2：加载常量
    fn load_constant(&self, layouter: impl Layouter<F>, constant: F) -> Result<Self::Num, Error>;
    /// 指令3：两个Num类型的乘法
    fn mul(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error>;
    /// 指令4：将一个数设置为电路的公共输出
    fn expose_public(
        &self,
        layouter: impl Layouter<F>,
        c: Self::Num,
        row: usize,
    ) -> Result<(), Error>;
}

///////////////////////////////////////////////////////////////////////
/// 定义芯片，在芯片结构中实现上面定义的指令集的trait（可以理解为接口）
///
/// 定义芯片配置结构 SimpleConfig，它是在配置过程中由芯片生成，存储在芯片内部
/// Plonk Configuation ccolumns: fixed, advice, instance
#[derive(Clone, Debug)]
struct SimpleConfig {
    //这里将用到两个 advice 列来实现自定义的指令集（advice: private + 中间值）
    advice: [Column<Advice>; 2],
    // public input (instance)
    instance: Column<Instance>,
    // 选择子，激活乘法门
    // 从而在用不到上面定义的 NumInstructions::mul指令的单元格上不设置任何约束
    s_mul: Selector,
}
/// 定义自定义芯片，芯片结构中包含了上面的配置，和一个占位符（https://rustwiki.org/zh-CN/std/marker/struct.PhantomData.html）
struct SimpleChip<F: Field> {
    config: SimpleConfig,
    _marker: PhantomData<F>,
}

/// 先为自定义的芯片实现 Chip trait（因为要实现 NumInstructions trait的类型必须先实现在F域上 Chip 的 trait）
/// 或者说，每一个芯片类型，都要实现 chip trait
/// Chip 的定义可见：https://docs.rs/halo2_proofs/latest/halo2_proofs/circuit/trait.Chip.html
/// Chip trait 定义了布局器在设计电路约束时，需要用到的列。trait中必须实现 Config, Loaded, config(), loaded()
impl<F: Field> Chip<F> for SimpleChip<F> {
    type Config = SimpleConfig;
    type Loaded = ();
    // 自定义的芯片配置
    fn config(&self) -> &Self::Config {
        &self.config
    }
    // 该芯片加载到电路所需要设置的初始状态，这里设置为空
    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

/// 实现自定义芯片的初始化和配置
impl<F: Field> SimpleChip<F> {
    // 默认构造方法
    fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
    /// 自定义配置：构建约束！
    /// 输入包括 fixed, advice, instance
    /// 约束包括：相等约束，选择器构建的乘法约束
    /// 返回多项式约束
    fn configure() {}
}

fn main() {
    println!("Hello, this is Halo2-Example: simple!");
}
