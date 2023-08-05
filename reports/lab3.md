# 实现方案
* git cherry-pick 移植之前的实现，将原来基于 TASK_MANAGER 的获取当前 task 的 API 迁移到 ch5 分离后的结构上
* 实现 spawn 时在 TaskControlBlock 添加新方法，结合 fork 和 exec。
* 实现 stride 调度，首先新增数据结构，记录 stride 和 pass，在 sys_set_priority 中更新 pass，在 suspend_current_and_run_next 更新 stride，在 TaskManager::fetch 中枚举找 stride 最小的任务