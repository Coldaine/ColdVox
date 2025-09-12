#!/usr/bin/env python3
"""
AT-SPI Accessibility Tree Logger
Captures all text fields and events from running applications
"""

import pyatspi
import sqlite3
import json
import hashlib
import time
from datetime import datetime
from pathlib import Path
from loguru import logger
from typing import Dict, Set, Optional, Any

class AccessibilityLogger:
    def __init__(self, db_path: str = "accessibility_log.db"):
        self.db_path = db_path
        self.init_database()
        self.object_cache: Dict[str, Dict] = {}
        self.text_hashes: Dict[str, str] = {}
        self.focused_objects: Set[str] = set()

        # Configure loguru for console output
        logger.remove()
        logger.add(
            lambda msg: print(msg, end=""),
            format="<green>{time:HH:mm:ss}</green> | <level>{level: <8}</level> | {message}",
            level="INFO"
        )
        logger.add("accessibility.log", rotation="10 MB", level="DEBUG")

    def init_database(self):
        """Initialize SQLite database with required tables"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Events table
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                app_name TEXT,
                object_id TEXT,
                object_role TEXT,
                object_name TEXT,
                detail1 INTEGER,
                detail2 INTEGER,
                source_info TEXT,
                UNIQUE(timestamp, event_type, object_id) ON CONFLICT IGNORE
            )
        """)

        # Text snapshots table (deduplicated by hash)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS text_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                object_id TEXT NOT NULL,
                app_name TEXT,
                object_role TEXT,
                object_name TEXT,
                text_content TEXT,
                text_hash TEXT NOT NULL,
                char_count INTEGER,
                can_read BOOLEAN,
                can_write BOOLEAN,
                interfaces TEXT,
                states TEXT,
                bounds TEXT,
                UNIQUE(object_id, text_hash) ON CONFLICT IGNORE
            )
        """)

        # Object registry (current state)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS object_registry (
                object_id TEXT PRIMARY KEY,
                app_name TEXT,
                object_role TEXT,
                object_name TEXT,
                last_seen TEXT,
                is_text_widget BOOLEAN,
                interfaces TEXT,
                states TEXT,
                bounds TEXT,
                last_text_hash TEXT
            )
        """)

        conn.commit()
        conn.close()

    def get_object_id(self, obj) -> str:
        """Generate stable ID for AT-SPI object"""
        try:
            # Use application name + role + path for stability
            app_name = obj.getApplication().name if obj.getApplication() else "unknown"
            role_name = obj.getRoleName()
            path = str(obj.path) if hasattr(obj, 'path') else str(id(obj))
            return f"{app_name}:{role_name}:{path}"
        except:
            return f"unknown:{id(obj)}"

    def get_object_info(self, obj) -> Dict[str, Any]:
        """Extract comprehensive info from AT-SPI object"""
        info = {
            'app_name': 'unknown',
            'role': 'unknown',
            'name': '',
            'interfaces': [],
            'states': [],
            'bounds': None,
            'text_content': '',
            'char_count': 0,
            'can_read': False,
            'can_write': False
        }

        try:
            # Basic properties
            if obj.getApplication():
                info['app_name'] = obj.getApplication().name or 'unknown'
            info['role'] = obj.getRoleName()
            info['name'] = obj.name or ''

            # Check interfaces
            try:
                text_interface = obj.queryText()
                info['interfaces'].append('Text')
                info['can_read'] = True
                info['text_content'] = text_interface.getText(0, -1)
                info['char_count'] = text_interface.characterCount
            except:
                pass

            try:
                obj.queryEditableText()
                info['interfaces'].append('EditableText')
                info['can_write'] = True
            except:
                pass

            try:
                component = obj.queryComponent()
                info['interfaces'].append('Component')
                bounds = component.getExtents(0)
                info['bounds'] = {
                    'x': bounds.x, 'y': bounds.y,
                    'width': bounds.width, 'height': bounds.height
                }
            except:
                pass

            try:
                obj.queryAction()
                info['interfaces'].append('Action')
            except:
                pass

            # States
            try:
                state_set = obj.getState()
                info['states'] = [str(state) for state in state_set.getStates()]
            except:
                pass

        except Exception as e:
            logger.debug(f"Error getting object info: {e}")

        return info

    def is_text_widget(self, obj) -> bool:
        """Check if object is a text-related widget"""
        try:
            role = obj.getRoleName().lower()
            text_roles = ['text', 'entry', 'password-text', 'paragraph', 'terminal', 'document']
            return role in text_roles
        except:
            return False

    def log_event(self, event):
        """Log AT-SPI event to database"""
        timestamp = datetime.now().isoformat()

        try:
            source_info = self.get_object_info(event.source)
            object_id = self.get_object_id(event.source)

            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()

            cursor.execute("""
                INSERT OR IGNORE INTO events
                (timestamp, event_type, app_name, object_id, object_role, object_name,
                 detail1, detail2, source_info)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                timestamp, event.type, source_info['app_name'], object_id,
                source_info['role'], source_info['name'],
                getattr(event, 'detail1', 0), getattr(event, 'detail2', 0),
                json.dumps(source_info)
            ))

            conn.commit()
            conn.close()

            logger.info(f"Event: {event.type} | {source_info['app_name']} | {source_info['role']}")

        except Exception as e:
            logger.error(f"Error logging event: {e}")

    def capture_text_snapshot(self, obj, triggered_by="manual"):
        """Capture and store text snapshot if changed"""
        if not self.is_text_widget(obj):
            return

        try:
            object_id = self.get_object_id(obj)
            info = self.get_object_info(obj)

            # Skip if no text content
            if not info['can_read'] or not info['text_content'].strip():
                return

            # Calculate content hash for deduplication
            text_hash = hashlib.sha256(info['text_content'].encode()).hexdigest()[:16]

            # Skip if we've seen this exact content for this object
            if self.text_hashes.get(object_id) == text_hash:
                return

            self.text_hashes[object_id] = text_hash
            timestamp = datetime.now().isoformat()

            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()

            cursor.execute("""
                INSERT OR IGNORE INTO text_snapshots
                (timestamp, object_id, app_name, object_role, object_name,
                 text_content, text_hash, char_count, can_read, can_write,
                 interfaces, states, bounds)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                timestamp, object_id, info['app_name'], info['role'], info['name'],
                info['text_content'], text_hash, info['char_count'],
                info['can_read'], info['can_write'],
                json.dumps(info['interfaces']), json.dumps(info['states']),
                json.dumps(info['bounds'])
            ))

            # Update registry
            cursor.execute("""
                INSERT OR REPLACE INTO object_registry
                (object_id, app_name, object_role, object_name, last_seen,
                 is_text_widget, interfaces, states, bounds, last_text_hash)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                object_id, info['app_name'], info['role'], info['name'], timestamp,
                True, json.dumps(info['interfaces']), json.dumps(info['states']),
                json.dumps(info['bounds']), text_hash
            ))

            conn.commit()
            conn.close()

            logger.success(f"📄 Captured text ({len(info['text_content'])} chars) from {info['app_name']} | {info['role']}")

        except Exception as e:
            logger.error(f"Error capturing text: {e}")

    def scan_all_applications(self):
        """Scan all running applications for text widgets"""
        logger.info("🔍 Scanning all applications...")

        try:
            desktop = pyatspi.Registry.getDesktop(0)

            for app_index in range(desktop.childCount):
                try:
                    app = desktop.getChildAtIndex(app_index)
                    app_name = app.name or "Unknown"

                    # Skip system apps
                    if app_name.lower() in ['at-spi-bus-launcher', 'gsd-media-keys', '']:
                        continue

                    logger.info(f"📱 Scanning {app_name}...")
                    self.scan_widget_tree(app)

                except Exception as e:
                    continue

        except Exception as e:
            logger.error(f"Error scanning applications: {e}")

    def scan_widget_tree(self, widget, depth=0, max_depth=10):
        """Recursively scan widget tree for text fields"""
        if depth > max_depth:
            return

        try:
            if self.is_text_widget(widget):
                self.capture_text_snapshot(widget, "scan")

            # Recurse to children
            for child_index in range(widget.childCount):
                try:
                    child = widget.getChildAtIndex(child_index)
                    self.scan_widget_tree(child, depth + 1, max_depth)
                except:
                    continue

        except Exception as e:
            pass

    def on_text_changed(self, event):
        """Handle text-changed events"""
        self.log_event(event)
        self.capture_text_snapshot(event.source, "text-changed")

    def on_focus_changed(self, event):
        """Handle focus events"""
        self.log_event(event)
        object_id = self.get_object_id(event.source)

        if event.detail1:  # Gained focus
            self.focused_objects.add(object_id)
            if self.is_text_widget(event.source):
                self.capture_text_snapshot(event.source, "focus")
        else:  # Lost focus
            self.focused_objects.discard(object_id)

    def on_generic_event(self, event):
        """Handle all other events"""
        self.log_event(event)

    def start_monitoring(self):
        """Start the event monitoring loop"""
        logger.info("🚀 Starting AT-SPI accessibility logger...")

        # Initial scan
        self.scan_all_applications()

        # Register event listeners
        pyatspi.Registry.registerEventListener(
            self.on_text_changed, "object:text-changed"
        )
        pyatspi.Registry.registerEventListener(
            self.on_focus_changed, "object:state-changed:focused"
        )
        pyatspi.Registry.registerEventListener(
            self.on_generic_event, "object:children-changed"
        )

        logger.info("✅ Event listeners registered. Press Ctrl+C to stop.")

        try:
            # Periodic rescans to catch missed objects
            def periodic_scan():
                while True:
                    time.sleep(30)  # Rescan every 30 seconds
                    logger.debug("🔄 Periodic rescan...")
                    self.scan_all_applications()

            import threading
            scan_thread = threading.Thread(target=periodic_scan, daemon=True)
            scan_thread.start()

            # Start the main event loop
            pyatspi.Registry.start()

        except KeyboardInterrupt:
            logger.info("🛑 Stopping logger...")
            pyatspi.Registry.stop()

if __name__ == "__main__":
    logger_instance = AccessibilityLogger()
    logger_instance.start_monitoring()
