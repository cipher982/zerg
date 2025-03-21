from zerg.app.database import engine
from zerg.app.models import Base


def create_tables():
    """Create all tables in the database"""
    Base.metadata.create_all(bind=engine)


if __name__ == "__main__":
    create_tables()
    print("Database tables created successfully!")
