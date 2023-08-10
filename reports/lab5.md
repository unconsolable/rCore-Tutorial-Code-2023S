# 实现方案
* 移植 copy_kernel_data，从而能够支持 sys_get_time。在测试用例中依赖这个系统调用
* 在 ProcessControlBlockInner 中定义 mutex 和 semaphore 的 available, allocation, need 向量。由于 thread, mutex, semaphore 的 id 不一定连续，因此使用 HashMap 表示向量。
* 死锁检测算法与描述相同，工作过程若在 allocation 找不到值，则取为 0，need 中找不到向量则认为是 0 向量
* 在 mutex 的 lock/unlock，semaphore 的 up/down 中按照描述修改 available, allocation, need 向量。
