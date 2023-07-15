from flask import Flask, jsonify

# Test server for tournament ID 35, game ID 1.
# Support members are part of the roster of each team.

app = Flask(__name__)


@app.route("/api/v1/tournaments/0/teams/10")
def ten():
    d = {
        "team": {
            "coaches": None,
            "division": "B",
            "flag_url": "https://uwhscores.com/static/flags/acc2023/George Mason.jpg",
            "name": "George Mason",
            "roster": [
                {
                    "name": " ",
                    "number": "",
                    "player_id": "Zenc38gS",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Abby Lloyd",
                    "number": 10,
                    "player_id": "s0wtkee8",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/76de9f9d-2dd5-434a-9e74-dee995c8fcd8_main.png",
                    "geared_picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "role": "man who dehypes",
                },
                {
                    "name": "Andres Toro",
                    "number": 17,
                    "player_id": "DuImekXY",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/26caac52-ade4-42a8-9330-6d5ed9fe842a_main.png",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Andrew Wright",
                    "number": None,
                    "player_id": "AAdiRiQe",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/d90ded9d-4d20-45c0-a90b-60246911a2fb_main.png",
                    "geared_picture_url": "",
                    "role": "man who hypes",
                },
                {
                    "name": "Brennen Leresche",
                    "number": 16,
                    "player_id": "bXXgaPo6",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/bac7f025-b931-4a8f-84a3-b2faf6dc5bb2_main.png",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Eileen Toussaint",
                    "number": 9,
                    "player_id": "NwW9kgni",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/913eaae9-bbaf-4fba-93d2-68175b336921_main.png",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Jackson Miller",
                    "number": 12,
                    "player_id": "22VIMHgZ",
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "geared_picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "role": None,
                },
                {
                    "name": "Kaya Shibuya",
                    "number": 23,
                    "player_id": "QKCTTxwi",
                    "picture_url": "",
                    "geared_picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "role": None,
                },
                {
                    "name": "Lucas Alm",
                    "number": 25,
                    "player_id": "LzCMibEh",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Matthew Sobelman",
                    "number": 5,
                    "player_id": "Oaow7LV3",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Noah Beck",
                    "number": 8,
                    "player_id": "weF99BsP",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Stephen Abernathy",
                    "number": 7,
                    "player_id": "hTWyRNtl",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
            ],
            "short_name": "George Mason",
            "team_id": 10,
        }
    }
    return jsonify(d)


@app.route("/api/v1/tournaments/0/teams/13")
def thirteen():
    d = {
        "team": {
            "coaches": None,
            "division": "B",
            "flag_url": "https://uwhscores.com/static/flags/acc2023/Texas.jpg",
            "name": "Texas",
            "roster": [
                {
                    "name": " ",
                    "number": "",
                    "player_id": "Zenc38gS",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": "dude",
                },
                {
                    "name": "Charles Cook is not a long name",
                    "number": 20,
                    "player_id": "hrYkxqdl",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Jacqui West",
                    "number": 14,
                    "player_id": "4hbiJbeB",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": "donut in charge",
                },
                {
                    "name": "Jordan Jacobson",
                    "number": 19,
                    "player_id": "EIYznqau",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Keith Morgan",
                    "number": 17,
                    "player_id": "Xxlmp4Lw",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Levi Charles Cook",
                    "number": 15,
                    "player_id": "glYnlSyz",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Nicholas Grande",
                    "number": 87,
                    "player_id": "mwFJlxWb",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Omri Svisa",
                    "number": 5,
                    "player_id": "YzCmV7Xd",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Ozkul Ege Akin",
                    "number": 7,
                    "player_id": "nHcmoTi5",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Quentin Scapoli",
                    "number": 6,
                    "player_id": "KcnDEAhL",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "Tristan Cook",
                    "number": 25,
                    "player_id": "fOgUK6aF",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "kevin barnes",
                    "number": 22,
                    "player_id": "pJ65PNdo",
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
            ],
            "short_name": "Texas",
            "team_id": 13,
        }
    }
    return jsonify(d)


@app.route("/api/v1/tournaments/0/games/1")
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
                {
                    "name": "joe",
                    "number": 0,
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "geared_picture_url": "",
                    "role": "Chief ref",
                },
                {
                    "name": "joke",
                    "number": None,
                    "picture_url": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                    "geared_picture_url": "",
                    "role": None,
                },
                {
                    "name": "jock",
                    "number": None,
                    "picture_url": "",
                    "geared_picture_url": "",
                    "role": None,
                },
            ],
            "white": "George Mason",
            "white_id": 10,
            "sponsor_logo": "https://w7.pngwing.com/pngs/381/29/png-transparent-logo-graphic-design-company-company-logo-angle-building-company.png",
        }
    }
    return jsonify(d)


app.run()
