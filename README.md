# ⚾️ pitchers 


Messing around with the MLB API pitch data.  Written in Rust 🦀

For example, looking at Game 2 of the 2025 World Series (Game id = `813026`)

```bash
$ cargo run  -- --game-pk 813026
```

```bash
Braydon Fisher ()
  slider       10
  curveball    2
  fastball     2

Jeff Hoffman ()
  slider       7
  Splitter     1

Kevin Gausman ()
  fastball     49
  Splitter     29
  slider       4

Louis Varland ()
  curveball    7
  fastball     5
  changeup     1

Yoshinobu Yamamoto ()
  curveball    36
  Splitter     34
  fastball     25
  slider       6
  sinker       4
```
