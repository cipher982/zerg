from datetime import datetime
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

from pydantic import BaseModel


# Agent schemas
class AgentBase(BaseModel):
    name: str
    system_instructions: str
    task_instructions: str
    model: str = "gpt-4o"
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None


class AgentCreate(AgentBase):
    pass


class AgentUpdate(BaseModel):
    name: Optional[str] = None
    system_instructions: Optional[str] = None
    task_instructions: Optional[str] = None
    model: Optional[str] = None
    status: Optional[str] = None
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None


class AgentMessage(BaseModel):
    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime

    class Config:
        orm_mode = True


class Agent(AgentBase):
    id: int
    status: str
    created_at: datetime
    updated_at: datetime
    messages: List[AgentMessage] = []

    class Config:
        orm_mode = True


# Message schemas
class MessageCreate(BaseModel):
    role: str
    content: str


class MessageResponse(BaseModel):
    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime

    class Config:
        orm_mode = True
