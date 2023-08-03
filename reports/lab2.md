# 实现方案
* 对于 sys_get_time 和 sys_task_info, 增加一个函数用来将内核数据结构拷贝为用户态的出参 `copy_kernel_data`，其思路和 `translated_byte_buffer` 类似，将数据结构转为一个 `&[u8]`，再逐页面写入。其他部分和之前实验相似。
* `MemorySet` 中增设找 `MapArea` 并在 PageTable 中删除 PTE 的函数，用于 unmap 函数
* `page_table` mod 中增加 `map_pages` 和 `unmap_pages` 函数，检查 `VPNRange` 范围情况，再调用 MemorySet 的方法
* 统计执行时间改用`get_time_us`，在 sys_task_info 中再转为 ms，减少误差