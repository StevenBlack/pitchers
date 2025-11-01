# ⚾️ pitchers 


Messing around with the MLB API pitch data.  Written in Rust 🦀

For example, looking at Game 6 of the 2025 World Series (Game id = `813026`)

```bash
$ cargo run  -- --id 813026
```

```bash
Braydon Fisher (14)
  heater  2
    fastball       2
  breaking ball 12
    slider        10
    curveball      2

Jeff Hoffman  (8)
  breaking ball  7
    slider         7
  offspeed  1
    splitter       1

Kevin Gausman (82)
  heater 49
    fastball      49
  breaking ball  4
    slider         4
  offspeed 29
    splitter      29

Louis Varland (13)
  heater  5
    fastball       5
  breaking ball  7
    curveball      7
  offspeed  1
    changeup       1

Yoshinobu Yamamoto (105)
  heater 42
    fastball      25
    cutter        13
    sinker         4
  breaking ball 29
    curveball     23
    slider         6
  offspeed 34
    splitter      34
```
