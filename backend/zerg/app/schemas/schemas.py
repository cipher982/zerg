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
    model: str
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


# Thread Message schemas
class ThreadMessageBase(BaseModel):
    role: str
    content: str
    tool_calls: Optional[List[Dict[str, Any]]] = None
    tool_call_id: Optional[str] = None
    name: Optional[str] = None


class ThreadMessageCreate(ThreadMessageBase):
    pass


class ThreadMessageResponse(ThreadMessageBase):
    id: int
    thread_id: int
    timestamp: datetime
    processed: bool = False

    class Config:
        orm_mode = True


# Thread schemas
class ThreadBase(BaseModel):
    title: str
    agent_state: Optional[Dict[str, Any]] = None
    memory_strategy: Optional[str] = "buffer"
    active: Optional[bool] = True


class ThreadCreate(ThreadBase):
    agent_id: int


class ThreadUpdate(BaseModel):
    title: Optional[str] = None
    agent_state: Optional[Dict[str, Any]] = None
    memory_strategy: Optional[str] = None
    active: Optional[bool] = None


class Thread(ThreadBase):
    id: int
    agent_id: int
    created_at: datetime
    updated_at: datetime
    messages: List[ThreadMessageResponse] = []

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
