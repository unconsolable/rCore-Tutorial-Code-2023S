# 实现方案
* 用哈希表存 syscall 次数（使用数组会出现 get_time 返回 0 的情况）
* 用 `Option<usize>` 表示首次启动时的时间，初始化时为 `None`，任务切换时若发现 `None`，则设置为 `Some(get_time_ms())`
* syscall dispatch 时更新系统调用次数

# 简答
* TBD
* 子问题
    * a0 代表第一个参数，对于陷入 trap 的场景则代表 trap context 的引用。__restore 用于从 trap 恢复，和首次启动 app 时初始化用户态
    * sstatus, sepc, sscratch. sstatus 表示 CPU 陷入 trap 前的特权级，sepc 表示 env call 返回后的下一条地址，sscratch 表示用户栈栈顶指针
    * x4 一般不会用到，x2 为栈指针，此时指向内核栈，用户栈指针在 sscratch
    * sp 表示用户栈栈顶指针, sscratch 表示内核栈栈顶指针
    * sret. 从 env call 中返回
    * sp 表示内核栈栈顶指针, sscratch 表示用户栈栈顶指针
    * ecall.
