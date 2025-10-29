# ⚾️ pitchers 


Messing around with the MLB API pitch data.  Written in Rust 🦀

For example, looking at Game 2 of the 2025 World Series (Game id = `813026`)

```bash
$ cargo run  -- --game-pk 813026
```

```bash
Braydon Fisher (14)
  slider       10
  fastball      2
  curveball     2

Jeff Hoffman  ( 8)
  slider        7
  Splitter      1

Kevin Gausman (82)
  fastball     49
  Splitter     29
  slider        4

Louis Varland (13)
  curveball     7
  fastball      5
  changeup      1

Yoshinobu Yamamoto (105)
  curveball    36
  Splitter     34
  fastball     25
  slider        6
  sinker        4
```
