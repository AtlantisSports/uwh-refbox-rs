pub(crate) const STYLE: &str = "
window {
    background-color: black;
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
button:active {
    background-color: #DD0000;
}
button.blue {
    background-color: #0000FF;
    color: #FFFFFF;
}
button.blue:active {
    background-color: #0000DD;
}
label.game-time {
    background-color: #000000;
    color: #FFFF00;
    font-size: 120px;
    font-weight: bold;
}";
