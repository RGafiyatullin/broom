# Broom

Broom is a minimalistic init-process that can spawn a single immediate child and reaps the indirect children it has to adopt.

Inspired by [the article](https://blog.phusion.nl/2015/01/20/docker-and-the-pid-1-zombie-reaping-problem/), driven by the itch to do some Rust.

## Sample

Let's say we run a docker-container with the following init-command:
```
sample-children/zombie-producer.sh 20 {10..5} {5..10}
```

That'll spawn quite a lot of child processes:
```
    1 pts/0    Ss+    0:00 sleep 20
    6 pts/0    S+     0:00 sleep 10
    7 pts/0    S+     0:00  \_ sleep 9
    8 pts/0    S+     0:00      \_ sleep 8
    9 pts/0    S+     0:00          \_ sleep 7
   10 pts/0    S+     0:00              \_ sleep 6
   11 pts/0    S+     0:00                  \_ sleep 5
   12 pts/0    S+     0:00                      \_ sleep 5
   13 pts/0    S+     0:00                          \_ sleep 6
   14 pts/0    S+     0:00                              \_ sleep 7
   15 pts/0    S+     0:00                                  \_ sleep 8
   16 pts/0    S+     0:00                                      \_ sleep 9
   17 pts/0    S+     0:00                                          \_ sleep 10
```


Some of them will terminate soon. Since their parent didn't `wait` its children the following picture emerges:
```
    1 pts/0    Ss+    0:00 sleep 20
    6 pts/0    Z+     0:00 [sleep] <defunct>
    7 pts/0    Z+     0:00 [sleep] <defunct>
    8 pts/0    Z+     0:00 [sleep] <defunct>
    9 pts/0    Z+     0:00 [sleep] <defunct>
   10 pts/0    Z+     0:00 [sleep] <defunct>
   11 pts/0    Z+     0:00 [sleep] <defunct>
   12 pts/0    Z+     0:00 [sleep] <defunct>
   13 pts/0    Z+     0:00 [sleep] <defunct>
   14 pts/0    Z+     0:00 [sleep] <defunct>
   15 pts/0    Z+     0:00 [sleep] <defunct>
   16 pts/0    Z+     0:00 [sleep] <defunct>
   17 pts/0    Z+     0:00 [sleep] <defunct>
```

... which is not good.



If we supply our program with a parent:
```
broom sample-children/zombie-producer.sh 20 {10..5} {5..10}
```

```
    1 pts/0    Ss+    0:00 target/release/broom sample-children/zombie-producer.sh 20 10 9 8 7 6 5 5 6 7 8 9 10
    6 pts/0    S+     0:00 sleep 20
    7 pts/0    S+     0:00  \_ sleep 10
    8 pts/0    S+     0:00      \_ sleep 9
    9 pts/0    S+     0:00          \_ sleep 8
   10 pts/0    S+     0:00              \_ sleep 7
   11 pts/0    S+     0:00                  \_ sleep 6
   12 pts/0    S+     0:00                      \_ sleep 5
   13 pts/0    S+     0:00                          \_ sleep 5
   14 pts/0    S+     0:00                              \_ sleep 6
   15 pts/0    S+     0:00                                  \_ sleep 7
   16 pts/0    S+     0:00                                      \_ sleep 8
   17 pts/0    S+     0:00                                          \_ sleep 9
   18 pts/0    S+     0:00                                              \_ sleep 10
```

The terminated children will be reaped in a timely manner:
```
    1 pts/0    Ss+    0:00 target/release/broom sample-children/zombie-producer.sh 20 10 9 8 7 6 5 5 6 7 8 9 10
    6 pts/0    S+     0:00 sleep 20
    7 pts/0    Z+     0:00  \_ [sleep] <defunct>
```
