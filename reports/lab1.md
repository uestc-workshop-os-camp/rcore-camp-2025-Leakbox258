1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。
   - SBI 版本信息：
     - ```[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0 [rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2```
   - `ch2b_bad_address.rs`: 尝试访问未映射的物理地址0x0
     - ```[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.```
   - `ch2b_bad_instructions.rs`: 尝试在U态使用S态指令`sret`
     - ```[kernel] IllegalInstruction in application, kernel killed it.```
   - `ch2b_bad_register.rs`: 尝试在U态访问S态寄存器`sstatus`
     - ```[kernel] IllegalInstruction in application, kernel killed it.```

2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

- 1. L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。
    - `sp` 指向内核栈顶
    - 在S返回U之前，恢复保存在栈上的寄存器
    - 在内核完成初始化之后，从S到U，开始执行任务
- 2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

```
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```
  - `sstatus`, `sepc`, `sscratch`
    - `sstatus`: 在`ssp`等字段指示在trap前，是U态还是S态
    - `sepc`: trap前最后一条指令的地址
    - `sscratch`: 在`__alltraps`开头和`sp`交换，此时存放用户态栈顶

- 3. L50-L56：为何跳过了 x2 和 x4？

```
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```
  - `x2(sp)` 需要特殊处理，`x4(tp)`一般不被使用
- 4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```
csrrw sp, sscratch, sp
```
  - 交换完毕之后，`sp` 重新指向trap前的用户栈，`sscratch` 指向内核栈

- 5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
  - `sret` 会切换 `sstatus` 的 `ssp` 字段以修改权限，并跳转至`sepc`, 返回用户代码空间
- 6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```
csrrw sp, sscratch, sp
```
  - 交换完毕之后，`sscratch` 重新指向trap前的用户栈，`sp` 指向内核栈
- 7. 从 U 态进入 S 态是哪一条指令发生的？
  - `sbi_call` 里的 `sbi_call`