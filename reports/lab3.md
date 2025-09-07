1. stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

 - 实际情况是轮到 p1 执行吗？为什么？
    - p2.stride + 10 后发生整形溢出，p2.stride 变成 4，下一次还是 p2 执行。

2. 我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

 - 为什么？尝试简单说明（不要求严格证明）。
   - 假设有两个进程 p1, p2，分别为当前的 STRIDE_MAX 和 STRIDE_MIN
   - STRIDE_MIN 调度之后，变为了 STRIDE_MIN ' = STRIDE_MIN + BigStride / p2.prio < STRIDE_MIN + BigStride / 2
   - 此时有 STRIDE_MAX - STRIDE_MIN ' < (STRIDE_MAX - STRIDE_MIN) - BigStride / 2
   - 即 STRIDE_MAX - STRIDE_MIN ' < BigStride / 2

3. 已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。

use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 < other.0 {
            Some(Ordering::Less)
        } else if self.0 > other.0 {
            Some(Ordering::Greater)
        } else {
            unreachable!()
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}