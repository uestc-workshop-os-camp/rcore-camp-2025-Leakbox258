1. 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？
   - 实际上起到的是root文件夹的direntry的作用，通过对该inode进行读写获取目录项
   - 无法正确按名查找root下的目录项，无法获取 block_id, block_offset 等信息，造成数据丢失
2. 举出使用 pipe 的一个实际应用的例子
   - ```cat output.txt | grep -a "Test passed" | grep -oP "\d{1,}/\d{1,}" | xargs -i echo "points={}"```