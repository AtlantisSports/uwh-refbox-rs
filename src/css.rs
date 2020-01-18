// Red = #FF0000
// Orange = FF8500
// Yellow = #FFFF00
// Green = #22FF00
// Lavender = #FF00FF
// Teal = #00FFFB
// Light Blue = #4E4EF4
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
grid.white {
    background-color: #FFFFFF;
    border-radius: 5px;
}
grid.black {
    background-color: #000000;
    border-radius: 5px;
}
grid.keypad {
    background-color: #696969;
    border-radius: 5px;
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
    font-size: 20px;
    font-weight: bold;
    border-style: solid;
    border-radius: 5px;
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
    background-color: #4E4EF4;
    color: #000000;
}
button.light-green {
    background-color: #38AA38;
    color: #000000;
}
button.blue {
    background-color: #0000FF;
    color: #FFFFFF;
    font-size: 30px;
}
button.blue-top {
    background-color: #0000FF;
    color: #FFFFFF;
    border-top-right-radius: 0px;
    border-top-left-radius: 0px;
    font-size: 30px;
}
button.grayed-out {
    background-color: #999999;
    color: #696969;
}
button.white-score {
    background-color: #FFFFFF;
    color: #000000;
    border-top-right-radius: 0px;
    border-top-left-radius: 0px;
    font-size: 40px;
    font-weight: bold;
}
button.black-score {
    background-color: #000000;
    color: #FFFFFF;
    border-top-right-radius: 0px;
    border-top-left-radius: 0px;
    font-size: 40px;
    font-weight: bold;
}
button.game-time {
    background-color: #696969;
    color: #22FF00;
    border-top-right-radius: 0px;
    border-top-left-radius: 0px;
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
button.time-mod {
    background-color: #D3D3D3;
    color: #000000;
}
button.time-edit {
    background-color: #4E4EF4;
    color: #000000;
}


label.game-state-header {
    background-color: #696969;
    color: #22FF00;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
    font-size: 10px;
    font-weight: bold;
}
label.white-header {
    background-color: #FFFFFF;
    color: #000000;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
    font-size: 10px;
    font-weight: bold;
}
label.black-header {
    background-color: #000000;
    color: #FFFFFF;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
    font-size: 10px;
    font-weight: bold;
}
label.game-time {
    background-color: #696969;
    color: #22FF00;
    font-size: 25px;
    font-weight: bold;
}
label.edit-white-score-header {
    background-color: #FFFFFF;
    color: #000000;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
    font-size: 10px;
    font-weight: bold;
}
label.edit-black-score-header {
    background-color: #000000;
    color: #FFFFFF;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
    font-size: 10px;
    font-weight: bold;
}
label.edit-parameter-header {
    background-color: #696969;
    color: #000000;
    border-radius: 5px;
}
label.edit-parameter-time {
    background-color: #D3D3D3;
    color: #000000;
}
label.player-number {
    background-color: #696969;
    color: #000000;
    border-top-right-radius: 5px;
    border-top-left-radius: 5px;
}
label.modified-score {
    background-color: #D3D3D3;
    color: #000000;
}
label.modified-white-score {
    background-color: #FFFFFF;
    color: #000000;
    border-bottom-right-radius: 5px;
    border-bottom-left-radius: 5px;
}
label.modified-black-score {
    background-color: #000000;
    color: #FFFFFF;
    border-bottom-right-radius: 5px;
    border-bottom-left-radius: 5px;
}
label.gray-top {
    background-color: #696969;
    color: #000000;
    border-top-right-radius: 0px;
    border-top-left-radius: 0px;
}


";
