from flask import Flask, jsonify

# Test server for tournament ID 35, game ID 1.

app = Flask(__name__)


@app.route("/api/v1/tournaments/35/teams/10")
def ten():
    d = {
        "team": {
            "coaches": None,
            "division": "B",
            "flag_url": "https://uwhscores.com/static/flags/acc2023/George Mason.jpg",
            "name": "George Mason",
            "support": [
                {
                    "name": "Cares Cok",
                    "role": "man who hypes",
                    "picture_url": "",
                },
                {
                    "name": "Jui Wes",
                    "role": "man who dehypes",
                    "picture_url": "",
                },
            ],
            "roster": [
                {
                    "name": " ",
                    "number": "",
                    "player_id": "Zenc38gS",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Abby Lloyd",
                    "number": 10,
                    "player_id": "s0wtkee8",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Andres Toro",
                    "number": 17,
                    "player_id": "DuImekXY",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Andrew Wright",
                    "number": 20,
                    "player_id": "AAdiRiQe",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Brennen Leresche",
                    "number": 16,
                    "player_id": "bXXgaPo6",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Eileen Toussaint",
                    "number": 9,
                    "player_id": "NwW9kgni",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Jackson Miller",
                    "number": 12,
                    "player_id": "22VIMHgZ",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Kaya Shibuya",
                    "number": 23,
                    "player_id": "QKCTTxwi",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Lucas Alm",
                    "number": 25,
                    "player_id": "LzCMibEh",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Matthew Sobelman",
                    "number": 5,
                    "player_id": "Oaow7LV3",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Noah Beck",
                    "number": 8,
                    "player_id": "weF99BsP",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Stephen Abernathy",
                    "number": 7,
                    "player_id": "hTWyRNtl",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
            ],
            "short_name": "George Mason",
            "team_id": 10,
        }
    }
    return jsonify(d)


@app.route("/api/v1/tournaments/35/teams/13")
def thirteen():
    d = {
        "team": {
            "coaches": None,
            "division": "B",
            "flag_url": "https://uwhscores.com/static/flags/acc2023/Texas.jpg",
            "name": "Texas",
            "support": [
                {
                    "name": "Cares Cok",
                    "role": "donut in charge",
                    "picture_url": "",
                },
                {
                    "name": "Juice East",
                    "role": "dude",
                    "picture_url": "",
                },
            ],
            "roster": [
                {
                    "name": " ",
                    "number": "",
                    "player_id": "Zenc38gS",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Charles Cook",
                    "number": 20,
                    "player_id": "hrYkxqdl",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Jacqui West",
                    "number": 14,
                    "player_id": "4hbiJbeB",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Jordan Jacobson",
                    "number": 19,
                    "player_id": "EIYznqau",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Keith Morgan",
                    "number": 17,
                    "player_id": "Xxlmp4Lw",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Levi Charles Cook",
                    "number": 15,
                    "player_id": "glYnlSyz",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Nicholas Grande",
                    "number": 87,
                    "player_id": "mwFJlxWb",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Omri Svisa",
                    "number": 5,
                    "player_id": "YzCmV7Xd",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Ozkul Ege Akin",
                    "number": 7,
                    "player_id": "nHcmoTi5",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Quentin Scapoli",
                    "number": 6,
                    "player_id": "KcnDEAhL",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "Tristan Cook",
                    "number": 25,
                    "player_id": "fOgUK6aF",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
                {
                    "name": "kevin barnes",
                    "number": 22,
                    "player_id": "pJ65PNdo",
                    "picture_url": "",
                    "geared_picture_url": "",
                },
            ],
            "short_name": "Texas",
            "team_id": 13,
        }
    }
    return jsonify(d)


@app.route("/api/v1/tournaments/35/games/1")
def hello():
    d = {
        "game": {
            "black": "Texas",
            "black_id": 13,
            "day": "Sat-8th",
            "description": None,
            "division": "B",
            "forfeit": None,
            "game_type": "RR-B",
            "gid": 1,
            "note_b": None,
            "note_w": None,
            "pod": "B",
            "pool": "1",
            "score_b": 7,
            "score_w": 1,
            "start_time": "2023-04-08T09:00:00",
            "tid": 35,
            "timing_rules": {
                "game_timeouts": {"allowed": 0, "duration": 30, "per_half": True},
                "half_duration": 540,
                "half_time_duration": 120,
                "max_sudden_death_duration": None,
                "min_game_break": 240,
                "overtime_allowed": False,
                "overtime_duration": 300,
                "pre_overtime_break": 120,
                "pre_sudden_death_break": 30,
                "sudden_death_allowed": False,
            },
            "referees": [
                {"name": "joe", "picture_url": ""},
                {"name": "joke", "picture_url": ""},
                {"name": "jock", "picture_url": ""},
            ],
            "white": "George Mason",
            "white_id": 10,
        }
    }
    return jsonify(d)


app.run()
