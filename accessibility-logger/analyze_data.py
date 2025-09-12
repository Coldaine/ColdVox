#!/usr/bin/env python3
"""
Analyze captured accessibility data
"""

import sqlite3
import pandas as pd
import json
from datetime import datetime

def analyze_database(db_path="accessibility_log.db"):
    conn = sqlite3.connect(db_path)

    print("=== Accessibility Data Analysis ===\n")

    # Summary stats
    print("📊 Summary Statistics:")
    events_count = conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]
    snapshots_count = conn.execute("SELECT COUNT(*) FROM text_snapshots").fetchone()[0]
    apps_count = conn.execute("SELECT COUNT(DISTINCT app_name) FROM object_registry").fetchone()[0]

    print(f"   Events logged: {events_count:,}")
    print(f"   Text snapshots: {snapshots_count:,}")
    print(f"   Apps monitored: {apps_count}")

    # Top apps by activity
    print("\n📱 Most Active Applications:")
    app_stats = conn.execute("""
        SELECT app_name, COUNT(*) as event_count
        FROM events
        GROUP BY app_name
        ORDER BY event_count DESC
        LIMIT 10
    """).fetchall()

    for app, count in app_stats:
        print(f"   {app}: {count:,} events")

    # Recent text captures
    print("\n📄 Recent Text Captures:")
    recent = conn.execute("""
        SELECT timestamp, app_name, object_role, char_count,
               substr(text_content, 1, 100) as preview
        FROM text_snapshots
        ORDER BY timestamp DESC
        LIMIT 10
    """).fetchall()

    for ts, app, role, chars, preview in recent:
        time_str = datetime.fromisoformat(ts).strftime("%H:%M:%S")
        preview_clean = preview.replace('\n', '\\n')
        print(f"   [{time_str}] {app} ({role}) - {chars} chars: '{preview_clean}'")

    # Text field types
    print("\n🔤 Text Field Types:")
    roles = conn.execute("""
        SELECT object_role, COUNT(*) as count
        FROM text_snapshots
        GROUP BY object_role
        ORDER BY count DESC
    """).fetchall()

    for role, count in roles:
        print(f"   {role}: {count:,} snapshots")

    conn.close()

if __name__ == "__main__":
    analyze_database()
