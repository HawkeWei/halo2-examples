# Halo2 Examples

## 预备知识

这里先简单介绍下Halo2中涉及到的结构和术语。

### Circuit

Circuit，电路。在零知识证明中的电路可以类比成集成电路，ZKP通过向电路中传入数据，经过电路门再输出，从而来创建证明。这里使用到了电路可满足问题。可以简单理解为，证明者只有知道一组满足电路的信号，才能正确通过电路。

### Chip

Chip，芯片。类似集成电路，zkp的电路也是由一个个芯片堆砌而成。
而每个芯片都需要初始化配置：config。

### Config

芯片的Config，就是申请芯片需要的 Column（advice\instance\fixed），定义自定义门电路或lookup table.

### Instructions

Instructions，指令。ZKP中的指令就是将Chip中实现的功能加入到电路中。每个芯片需要实现其需要的指令接口，电路则调用芯片实现的指令来完成指定功能。

### Layouter

Layouter，布局器。电路由一个个芯片堆砌而成，但不是随便堆砌，这里就需要布局器在电路上进行芯片的布局。电路上的布局可以分层（可以理解为芯片堆叠，分为多层）。

### Assignment

Assignment是一个电路赋值的接口。

## 使用halo2实现的一些Examples

### [simple example](./simple/src/main.rs)

实现 a^2 * b^2 = c

### [fabonacci](./fibonacci/src/main.rs)

参考 [0xparc Halo2 课程](https://learn.0xparc.org/materials/halo2/learning-group-1/halo2-api)

## 附录：Halo2 资料整理

- [halo2 book](https://zcash.github.io/halo2/design/proving-system.html)
- [halo2 book 中文版](https://trapdoor-tech.github.io/halo2-book-chinese/)
- [halo2 - PSE](https://github.com/privacy-scaling-explorations/halo2)
- [halo2 solidity verifier - PSE](https://github.com/privacy-scaling-explorations/halo2-solidity-verifier)
- [halo2 入门 - axiom](https://docs.axiom.xyz/zero-knowledge-proofs/getting-started-with-halo2)
- [halo2 lib - axiom](https://github.com/axiom-crypto/halo2-lib)
- [halo2 circuit tools - taikoxyz](https://github.com/taikoxyz/circuit-tools)
