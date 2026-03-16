import logging
import logging.config
import os

log = logging.getLogger("setup_api")
logging.basicConfig(level=logging.INFO)

config = {
    "version": 1,
    "disable_existing_loggers": True,
    "formatters": {
        "simple": {
            "format": "%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        },
        "json": {
            "format": "%(asctime)s - %(name)s - %(levelname)s - %(message)s",
            "class": "pythonjsonlogger.jsonlogger.JsonFormatter",
        },
    },
    "handlers": {
        "console": {
            "class": "logging.StreamHandler",
            "formatter": "json",
            "level": "DEBUG",
            "stream": "ext://sys.stdout",
        },
    },
    "loggers": {
        "root": {
            "level": "DEBUG",
            "handlers": ["console"],
        },
        "setup_api": {
            "level": "DEBUG",
            "handlers": ["console"],
            "propagate": False,
        },
        "uvicorn": {"level": "DEBUG", "handlers": ["console"], "propagate": False},
        "uvicorn.error": {
            "level": "DEBUG",
            "handlers": ["console"],
            "propagate": False,
        },
        "uvicorn.access": {
            "level": "DEBUG",
            "handlers": ["console"],
            "propagate": False,
        },
    },
}

if os.environ.get("LOG_LOCATION"):
    config["handlers"]["file"] = {
        "class": "logging.FileHandler",
        "formatter": "json",
        "level": "DEBUG",
        "filename": os.environ.get("LOG_LOCATION"),
    }
    config["loggers"]["root"]["handlers"] = ["console", "file"]
    config["loggers"]["setup_api"]["handlers"] = ["console", "file"]
    config["loggers"]["uvicorn"]["handlers"] = ["console", "file"]
    config["loggers"]["uvicorn.error"]["handlers"] = ["console", "file"]
    config["loggers"]["uvicorn.access"]["handlers"] = ["console", "file"]

logging.config.dictConfig(config)
