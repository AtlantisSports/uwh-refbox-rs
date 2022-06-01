# Game Snapshot encoding

The entire game snapshot is encoded into 19 bytes of data, with the limitation
that only up to three penalties of each color can be sent. The byte stream
always has space for all 6 penalties, but the penalty encoding includes data on
whether that penalty is actually present, or just a placeholder. The bytestream
is organized as follows:

| Byte(s) | Description    |
| ------- | -------------- |
| 18:17   | w_penalty_2    |
| 16:15   | w_penalty_1    |
| 14:13   | w_penalty_0    |
| 12:11   | b_penalty_2    |
| 10:9    | b_penalty_1    |
| 8:7     | b_penalty_0    |
| 6       | w_score        |
| 5       | b_score        |
| 4:3     | timeout        |
| 2:1     | secs_in_period |
| 0       | period_info    |


# Period encoding

The `current_period` and `is_old_game` values are encoded together in 8 bits as follows:

| Bit(s) | Description                                                                          |
| ------ | ------------------------------------------------------------------------------------ |
| 7      | `is_old_game`                                                                        |
| 4:0    | `current_period`: values 0-9, arranged in order from `BetweenGames` to `SuddenDeath` |


# Timeout encoding

The timeout state is encoded in a 16 bit value as follows:

| Bit(s) | Description                                                                                                                              |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| 15:13  | Timeout type. Allowed values:<br>  - 0b000: None<br>  - 0b001: Black<br>  - 0b010: White<br>  - 0b011: Ref<br>  - 0b100: Penalty Shot    |
| 12:0   | Time remaining in timeout (seconds).<br>Max allowed value:<br>  5999<br>  (largest value that can be<br>  displayed in the mm:ss format) |


# Penalty encoding

Each penalty is encoded as a 16 bit value as follows:

| Bit(s) | Description                                                                                                      |
| ------ | ---------------------------------------------------------------------------------------------------------------- |
| 15:9   | Player number. Possible values:<br>  - 0-99: valid player number<br>  - 100-126: Reserved<br>  - 127: No penalty |
| 8:0    | Time remaining in penalty (seconds).<br>  - 0-510 indicate valid times<br>  - 511 indicates total dismissal      |
