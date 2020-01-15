// Red = #FF0000
// Orange = FF8500
// Yellow = #FFFF00
// Green = #22FF00
// Lavender = #FF00FF
// Teal = #00FFFB
// Light Blue = #5555F5
// Light Green = #38AA38
// Background Gray = #D3D3D3
// Grayed-Out Gray = #999999
// Dark Gray = #696969
// Black = #000000
// White = #FFFFFF

pub(crate) const STYLE: &str = " 
window {
    background-color: #D3D3D3;
}
button {
    border-style: none;
    outline-style: none;
    box-shadow: none;
    background-color: #FF0000;
    color: #000000;
    background-image: none;
    text-shadow: none;
    -gtk-icon-shadow: none;
    -gtk-icon-effect: none;
}
button:hover {
    -gtk-icon-shadow: none;
    -gtk-icon-effect: none;
}
button.keypad {
    background-color: #00FFFB;
    color: #000000;
    font-size: 50px;
    font-weight: bold;
    border-radius: 0px;
    border-width: 2px;
    border-color: #696969;
}
button.blue:active {
    background-color: #0000DD;
}
button:active {
    background-color: #DD0000;
}
button.white {
    background-color: #FFFFFF;
    color: #000000;
}
button.black {
    background-color: #000000;
    color: #FFFFFF;
}
button.dark-gray {
    background-color: #696969;
    color: #22FF00;
}
button.red {
    background-color: #FF0000;
    color: #000000;
}
button.orange {
    background-color: #FF8500;
    color: #000000;
}
button.yellow {
    background-color: #FFFF00;
    color: #000000;
}
button.green {
    background-color: #22FF00;
    color: #000000;
}
button.lavender {
    background-color: #FF00FF;
    color: #000000;
}
button.light-blue {
    background-color: #5555F5;
    color: #000000;
}
button.light-green {
    background-color: #38AA38;
    color: #000000;
}
button.blue {
    background-color: #0000FF;
    color: #FFFFFF;
    font-size: 80px;
}
button.grayed-out {
    background-color: #999999;
    color: #696969;
}
button.white-score {
    background-color: #FFFFFF;
    color: #000000;
    font-size: 40px;
    font-weight: bold;
}
button.black-score {
    background-color: #000000;
    color: #FFFFFF;
    font-size: 40px;
    font-weight: bold;
}
button.game-time {
    background-color: #696969;
    color: #22FF00;
    font-size: 40px;
    font-weight: bold;
}
button.gray {
    background-color: #999999;
    color: #000000;
}
button.blue:active {
    background-color: #0000DD;
}





label.game-state-header {
    background-color: #696969;
    color: #22FF00;
    font-size: 10px;
    font-weight: bold;
}
label.white-header {
    background-color: #FFFFFF;
    color: #000000;
    font-size: 10px;
    font-weight: bold;
}
label.black-header {
    background-color: #000000;
    color: #FFFFFF;
    font-size: 10px;
    font-weight: bold;
}
label.game-time {
    background-color: #696969;
    color: #22FF00;
    font-size: 25px;
    font-weight: bold;
}
label.time-mod {
    background-color: #D3D3D3;
    color: #000000;
}
label.player-number {
    background-color: #D3D3D3;
    color: #000000;
}
label.new_time_header {
    background-color: #999999;
    color: #000000;
}
label.modified-game-time {
    background-color: #999999;
    color: #000000;
}";
