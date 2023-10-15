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
/// 这是学习 halo2 的第一个应用例子，主要用来熟悉 zcash-halo2 所提供的API。
/// 解析参考：https://learnblockchain.cn/article/3442
/// 例子用来计算和证明 a^2 * b^2 = c, 其中 a、b 为 private input，c 为 public input
///

///////////////////////////////////////////////////////////////////////
/// 1、定义自定义的指令集，本例中指令包括4个指令：加载私有变量， 加载常量， 计算2个数的乘法，导出公共输入
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
/// 2、定义芯片，在芯片结构中实现上面定义的指令集的trait（可以理解为接口）
/// 电路由一个个Chip逻辑堆砌而成。每个Chip的创建从 Config 开始。
/// 定义芯片配置结构 SimpleConfig（Config 就是申请Chip需要的Column以及配置Fixed列的逻辑含义。这些配置可能是Custom Gate，可能是lookup）
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

///////////////////////////////////////////////////////////////////////
/// 3、实现自定义芯片的配置
/// Configure调用ConstraintSystem申请各种列以及Gate的信息。
/// 调用某个Circuit的Configure函数会顺序调用电路涉及到的Chip的Configure信息，这些信息都记录在ConstraintSystem中。
impl<F: Field> SimpleChip<F> {
    // 默认构造方法
    fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
    /// 自定义配置：构建约束！
    /// 输入包括 advice, instance, fixed
    /// 约束包括：相等约束，选择器构建的乘法约束
    /// 返回多项式约束
    fn configure(
        meta: &mut ConstraintSystem<F>, // 约束系统：这是对电路环境的描述，例如门、列和排列的安排。
        advice: [Column<Advice>; 2],    // private input + 中间值
        instance: Column<Instance>,     // public input
        constant: Column<Fixed>,        // selector
    ) -> SimpleConfig {
        // 启用强制执行指定列中的单元格相等的功能
        meta.enable_equality(instance);
        for c in &advice {
            meta.enable_equality(*c);
        }
        // 使该固定列能够用于全局常量赋值。此外，该列也将默认启用 enable_equality
        meta.enable_constant(constant);
        // 选择器，激活乘法门
        let s_mul = meta.selector();

        /// 定义乘法门
        /// create_gate 返回多项式表达式的约束，在证明系统中一定等于0
        meta.create_gate("mul", |meta| {
            // 这里需要3个 advice cells 和 1个 selector cell 来实现乘法
            // 参考官方案例，把他们按下表来排列：
            //
            // | a0  | a1  | s_mul |
            // |-----|-----|-------|
            // | lhs | rhs | s_mul |
            // | out |     |       |
            // 门可以用任一相对偏移，但每一个不同的偏移都会对证明增加开销。
            // 最常见的偏移值是 0 (当前行), 1(下一行), -1(上一行)。
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());
            let s_mul = meta.query_selector(s_mul);

            /// 当不是乘法时，s_mul为0，lhs、rhs、out可以时任何值，返回仍为0
            /// 当是乘法时，s_mul为1，lhs、rhs、out必须满足 lhs * rhs - out = 0 的约束
            vec![s_mul * (lhs * rhs - out)]
        });
        SimpleConfig {
            advice,
            instance,
            s_mul,
        }
    }
}

///////////////////////////////////////////////////////////////////////
/// 4、实现芯片核心功能
/// 除了实现simpleChip本身的方法之外，还需要为SimpleShip实现自定义的 NumInstructions Trait，
/// 以及所有芯片必须实现的 Chip trait
///
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

/// 接着为 SimpleChip实现自定义的 NumInstructions Trait
/// 先定义一个Number 结构体，表示指定的单元格
// #[derive(Clone)]
// struct Number<F: Field>(AssignedCell<F, F>);
impl<F: Field> NumInstructions<F> for SimpleChip<F> {
    type Num = AssignedCell<F, F>;
    fn load_private(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
    ) -> Result<Self::Num, Error> {
        let config = self.config();
        layouter.assign_region(
            || "load_private",
            |mut region| region.assign_advice(|| "private input", config.advice[0], 0, || a),
        )
    }

    fn load_constant(
        &self,
        mut layouter: impl Layouter<F>,
        constant: F,
    ) -> Result<Self::Num, Error> {
        let config = self.config();
        layouter.assign_region(
            || "load_constant",
            |mut region| {
                region.assign_advice_from_constant(
                    || "constant value",
                    config.advice[0],
                    0,
                    constant,
                )
            },
        )
    }

    fn mul(
        &self,
        mut layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error> {
        let config = self.config();
        layouter.assign_region(
            || "mul",
            |mut region| {
                /// 在这个芯片区域中，我们只使用了乘法，所以只需要激活 s_mul
                config.s_mul.enable(&mut region, 0);
                /// 官方解释：给我们的输入(a: Self::Num / b: Self::Num,) 有可能在电路的任何位置.
                /// 但在region 中，我们只能依靠相对偏移。所以我们需要在 region 内分配新的 cells
                /// 并限制新分配的 cells 的值 与输入(a: Self::Num / b: Self::Num,) 的值相等。
                /// 即使用拷贝约束？
                a.copy_advice(|| "lhs", &mut region, config.advice[0], 0);
                b.copy_advice(|| "rhs", &mut region, config.advice[1], 0);
                /// 计算乘积
                let res = a.value().copied() * b.value();
                /// 对输出赋值，cell所在位置在config中定义过，这里使用相对位置定位
                region.assign_advice(|| "lhs * rhs", config.advice[0], 1, || res)
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        c: Self::Num,
        row: usize,
    ) -> Result<(), Error> {
        let config = self.config();
        layouter.constrain_instance(c.cell(), config.instance, row)
    }
}

///////////////////////////////////////////////////////////////////////
/// 5、构建电路
/// 上面步骤中，已经进行了自定义指令、定义芯片、实现芯片的过程，接下来就是构建电路
/// 自定义电路中需要实现 plonk::Circuit 的 trait （https://docs.rs/halo2_proofs/latest/halo2_proofs/plonk/trait.Circuit.html）
///
/// 首先先定义电路结构体. 结构体中保存private input
/// 官方解释：我们使用 `Option<F>` 类型是因为，
/// 在生成密钥阶段，它们不需要有任何的值；在证明阶段中，如果它们任一为 `None` 的话，我们将得到一个错误。
#[derive(Default)]
struct SimpleCircuit<F: Field> {
    constant: F,
    a: Value<F>,
    b: Value<F>,
}

///////////////////////////////////////////////////////////////////////
/// 为 SimpleCircuit实现 Circuit trait
/// 在电路实现时，
/// 1）先进行configure过程：Circuit configure -> Chip configure -> ConstraintSystem，最后返回 CircuitConfig
/// 2）再进行Synthesize过程：
impl<F: Field> Circuit<F> for SimpleCircuit<F> {
    type Config = SimpleConfig;
    type FloorPlanner = SimpleFloorPlanner;

    /// 返回此电路的副本，没有 witness（即所有witness设置为 None）。对于大多数电路，这将等于Self::default()。
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    /// 精确的布置电路门、列的排列
    /// 输入约束系统，输出之前自定义的 simpleConfig
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        // 2个 advice 列，存储 private input
        let advice = [meta.advice_column(), meta.advice_column()];
        // 1个 instance 列，存储 public input
        let instance = meta.instance_column();
        // fixed列，储存常数
        let constant = meta.fixed_column();

        // 调用芯片的配置，初始化配置
        SimpleChip::configure(meta, advice, instance, constant)
    }

    /// 根据提供的 config，来对 Layouter 进行赋值，核心用到了它的 assin_region() 函数，而这个函数用到了 closure，它的参数是 Region。
    /// 这里直接调用我们在simpleChip中实现的的4个指令（load_private, load_constant, mul, expose_public）
    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let simple_chip = SimpleChip::<F>::construct(config);

        /// 将 private input 加载到电路中
        /// ? 运算符用在返回值为 Result 的表达式后面，它等同于这样一个匹配表达式：其中 Err(err) 分支展开成提前返回的 return Err(err)，而 Ok(ok) 分支展开成 ok 表达式。
        let a = simple_chip.load_private(layouter.namespace(|| "load a"), self.a)?;
        let b = simple_chip.load_private(layouter.namespace(|| "load b"), self.b)?;

        /// 将常数加载到电路中
        let constant =
            simple_chip.load_constant(layouter.namespace(|| "load constant"), self.constant)?;

        /// 实现 a^2 * b^2
        let a2 = simple_chip.mul(layouter.namespace(|| "a * a"), a.clone(), a)?;
        let b2 = simple_chip.mul(layouter.namespace(|| "b * b"), b.clone(), b)?;
        let a2_b2 = simple_chip.mul(layouter.namespace(|| "a^2 * b^2"), a2, b2)?;
        let c = simple_chip.mul(
            layouter.namespace(|| "constant * a^2 * b^2"),
            constant,
            a2_b2,
        )?;

        /// 把运算结果作为电路的public input
        simple_chip.expose_public(layouter.namespace(|| "expose c"), c, 0)
    }
}

fn main() {
    println!("Hello, this is halo2 example: simple example...");
    // 定义电路的行数
    let row = 5;

    // 隐私输入和常数
    let constant = Fp::from(2);
    let a = Fp::from(2);
    let b = Fp::from(3);

    // 用隐私输入实例化电路
    let my_circuit = SimpleCircuit {
        constant,
        a: Value::known(a), // 构造一个已知值
        b: Value::known(b), // 构造一个已知值
    };

    // 计算正确的公共输入，并将乘法的结果放置在 instance 列的第0行
    let c = constant * a.square() * b.square();
    let mut public_input = vec![c];

    /// 使用开发包中调试电路的测试验证器 MockProver（https://docs.rs/halo2_proofs/latest/halo2_proofs/dev/struct.MockProver.html）
    /// MockProver::run ：在给定电路上运行合成密钥生成和证明操作，收集有关约束及其分配的数据
    let prover1 = MockProver::run(row, &my_circuit, vec![public_input]).unwrap();
    /// MockProver::cerify : Ok(())如果满足则返回MockProver，或者指示电路不满足的原因的错误列表
    let res1 = prover1.verify();
    println!("res1: {:?}", res1);

    /// 使用错误的 public input（没有乘以常数）
    /// 将会验证失败
    let d = a.square() * b.square();
    public_input = vec![d];
    let prover2 = MockProver::run(row, &my_circuit, vec![public_input]).unwrap();
    let res2 = prover2.verify();
    println!("res2: {:?}", res2);
}
