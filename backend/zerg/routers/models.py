from fastapi import APIRouter

from zerg.models_config import get_all_models_for_api

router = APIRouter(
    tags=["models"],
)


@router.get("/")
async def get_models():
    """Return available models."""
    return get_all_models_for_api()
