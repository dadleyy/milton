target remote :3333
set remote hardware-watchpoint-limit 10
mon reset halt
flushregs
break src/main.rs:82
# thb main
# c
