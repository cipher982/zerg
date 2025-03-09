from sqlalchemy import JSON
from sqlalchemy import Column
from sqlalchemy import DateTime
from sqlalchemy import ForeignKey
from sqlalchemy import Integer
from sqlalchemy import String
from sqlalchemy import Text
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func

from zerg.app.database import Base


class Agent(Base):
    __tablename__ = "agents"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String, nullable=False)
    status = Column(String, default="idle")
    instructions = Column(Text)
    schedule = Column(String, nullable=True)  # CRON expression or interval
    model = Column(String, default="gpt-4o")  # Default model to use
    config = Column(JSON, nullable=True)  # Additional configuration as JSON
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())

    # Define relationship with AgentMessage
    messages = relationship("AgentMessage", back_populates="agent", cascade="all, delete-orphan")


class AgentMessage(Base):
    __tablename__ = "agent_messages"

    id = Column(Integer, primary_key=True, index=True)
    agent_id = Column(Integer, ForeignKey("agents.id"))
    role = Column(String, nullable=False)  # "system", "user", "assistant"
    content = Column(Text, nullable=False)
    timestamp = Column(DateTime, server_default=func.now())

    # Define relationship with Agent
    agent = relationship("Agent", back_populates="messages")
