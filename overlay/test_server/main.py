from flask import Flask, jsonify, request

app = Flask(__name__)


@app.route("/api/admin/get-event-team")
def team():
    tournament = request.args.get("legacyEventId")
    team = request.args.get("legacyTeamId")
    if team == "13":
        d = {
            "logoUrl": "https://uwhscores.blob.core.windows.net/images/408c31e8-3351-456e-a0b9-9fb4de18ddc7_main.jpg",
            "name": "Door County Salty Sturgeons",
            "photos": [],
            "roster": [
                {
                    "capNumber": 23,
                    "photos": {
                        "darkGear": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                        "lightGear": None,
                        "uniform": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    },
                    "roles": ["Player", "Coach"],
                    "rosterName": "Susan Banks",
                },
                {
                    "capNumber": 22,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Trisha Filar",
                },
                {
                    "capNumber": 88,
                    "photos": {
                        "darkGear": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                        "lightGear": None,
                        "uniform": None,
                    },
                    "roles": ["Player"],
                    "rosterName": "David Foulds",
                },
                {
                    "capNumber": 27,
                    "photos": {
                        "darkGear": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                        "lightGear": None,
                        "uniform": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    },
                    "roles": ["Player"],
                    "rosterName": "Peter Covach",
                },
                {
                    "capNumber": 60,
                    "photos": {
                        "darkGear": "https://underwaterrugby.blob.core.windows.net/images/43226f46-5f89-4162-9b94-ecbadf6b4be0_main.png",
                        "lightGear": None,
                        "uniform": None,
                    },
                    "roles": ["Player"],
                    "rosterName": "Matt Brown",
                },
                {
                    "capNumber": 7,
                    "photos": {
                        "darkGear": None,
                        "lightGear": None,
                        "uniform": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    },
                    "roles": ["Player"],
                    "rosterName": "Richard Heller",
                },
                {
                    "capNumber": 99,
                    "photos": {
                        "darkGear": None,
                        "lightGear": None,
                        "uniform": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    },
                    "roles": ["Player"],
                    "rosterName": "Craig Hutchinson",
                },
                {
                    "capNumber": 39,
                    "photos": {
                        "darkGear": None,
                        "lightGear": None,
                        "uniform": "https://underwaterrugby.blob.core.windows.net/images/6667b743-0413-4067-b418-526c896e3422_main.png",
                    },
                    "roles": ["Player"],
                    "rosterName": "Sean Linnan",
                },
                {
                    "capNumber": 11,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Steven Shephard",
                },
                {
                    "capNumber": 29,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Justin Therrian",
                },
                {
                    "capNumber": 50,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Albert Weber III",
                },
            ],
        }
    else:
        d = {
            "logoUrl": "https://uwhscores.blob.core.windows.net/images/d7a68495-f2b5-4403-8788-ac9246f03867_main.jpg",
            "name": "Team Oregon",
            "photos": [],
            "roster": [
                {
                    "capNumber": 8,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 2,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 7,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 4,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 13,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Jenny Mohn",
                },
                {
                    "capNumber": 14,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 9,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 6,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 11,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 12,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 3,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": None,
                },
                {
                    "capNumber": 1,
                    "photos": {"darkGear": None, "lightGear": None, "uniform": None},
                    "roles": ["Player"],
                    "rosterName": "Kat Kurtz",
                },
            ],
        }
    return jsonify(d)


@app.route("/api/v1/tournaments/0/games/1")
def hello():
    d = {
        "game": {
            "black": "Team Oregon",
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
            "white": "Door County Salty Sturgeons",
            "white_id": 10,
            "sponsor_logo": "https://w7.pngwing.com/pngs/381/29/png-transparent-logo-graphic-design-company-company-logo-angle-building-company.png",
        }
    }
    return jsonify(d)


app.run()
