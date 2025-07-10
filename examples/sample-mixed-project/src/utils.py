#!/usr/bin/env python3
"""
Utility functions for the sample project
Demonstrates Python code in a mixed-language project
"""

import json
import hashlib
from datetime import datetime
from typing import Dict, List, Any


def generate_hash(data: str) -> str:
    """Generate SHA256 hash of input data."""
    return hashlib.sha256(data.encode()).hexdigest()


def parse_config(config_path: str) -> Dict[str, Any]:
    """Parse configuration from JSON file."""
    try:
        with open(config_path, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        return {}
    except json.JSONDecodeError as e:
        print(f"Error parsing config: {e}")
        return {}


class DataProcessor:
    """Process and transform data for the application."""
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.processed_count = 0
    
    def process_batch(self, items: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Process a batch of items."""
        results = []
        for item in items:
            processed = self._process_item(item)
            results.append(processed)
            self.processed_count += 1
        return results
    
    def _process_item(self, item: Dict[str, Any]) -> Dict[str, Any]:
        """Process a single item."""
        return {
            **item,
            'processed_at': datetime.utcnow().isoformat(),
            'hash': generate_hash(json.dumps(item)),
            'version': self.config.get('version', '1.0.0')
        }


if __name__ == "__main__":
    # Example usage
    sample_data = [
        {'id': 1, 'name': 'Item 1'},
        {'id': 2, 'name': 'Item 2'}
    ]
    
    processor = DataProcessor({'version': '1.0.0'})
    results = processor.process_batch(sample_data)
    
    print(json.dumps(results, indent=2))