# 实现方案
* 修改 easyfs DiskInode，增加记录链接数的 hard_link，初始化为 1。linkat 时加一。
* 修改 easyfs Inode，增加记录 inode_id，并增设 Stat struct，用于 fstat。
* linkat 时找到文件 inode id，hard_link 更新，写入 dirent。
* unlinkat 时找到文件 inode id，hard_link 更新，dirent 写入一个无效 inode_id 表示已删除。无效 id 用的是 root id（目前没有对根目录的链接）。
* stat 会在 File trait 增加新方法，从而获取 stat