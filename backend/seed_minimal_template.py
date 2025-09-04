#!/usr/bin/env python3
"""
Seed script to create the "minimal" workflow template.
Run this after setting up the database to ensure the minimal template exists.
"""

import uuid

from zerg.crud import crud
from zerg.database import db_session


def create_minimal_template():
    """Create the minimal workflow template with a manual trigger."""

    # Define the minimal template canvas with a single Manual Trigger node
    minimal_canvas = {
        "nodes": [
            {
                "node_id": str(uuid.uuid4()),
                "node_type": "Trigger",
                "config": {
                    "x": 400.0,
                    "y": 200.0,
                    "width": 200.0,
                    "height": 80.0,
                    "color": "#10b981",
                    "text": "Manual Trigger",
                    "trigger_type": "Manual",
                    "enabled": True,
                    "params": {},
                    "filters": [],
                },
                "position": {"x": 400.0, "y": 200.0},
            }
        ],
        "edges": [],
    }

    with db_session() as db:
        # Check if minimal template already exists
        existing = crud.get_workflow_template_by_name(db=db, template_name="minimal")
        if existing:
            print(f"✓ Minimal template already exists with ID: {existing.id}")
            return existing

        # Create the minimal template directly using SQL to avoid ORM field issues
        import json

        from sqlalchemy import text

        result = db.execute(
            text("""
            INSERT INTO workflow_templates 
            (created_by, name, description, category, canvas, tags, 
             preview_image_url, is_public, created_at, updated_at)
            VALUES 
            (1, 'minimal', 'Minimal workflow template with a manual trigger for getting started', 
             'starter', :canvas, '[]', NULL, 1, datetime('now'), datetime('now'))
        """),
            {"canvas": json.dumps(minimal_canvas)},
        )

        template_id = result.lastrowid
        db.commit()

        # Fetch the created template through ORM
        from zerg.models.models import WorkflowTemplate

        template = db.query(WorkflowTemplate).filter_by(id=template_id).first()

        print(f"✓ Created minimal template with ID: {template.id}")
        return template


if __name__ == "__main__":
    print("🌱 Creating minimal workflow template...")
    template = create_minimal_template()
    print(f"🎉 Minimal template ready: '{template.name}' (ID: {template.id})")
