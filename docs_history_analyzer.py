#!/usr/bin/env python3
"""
Comprehensive Documentation History Analyzer for Coldvox Repository

This script analyzes the evolution of documentation in the Git repository by:
1. Extracting git history for all markdown files
2. Processing and structuring the data
3. Analyzing changes between versions
4. Creating visualizations of documentation evolution
5. Cross-referencing with code changes and releases
"""

import os
import subprocess
import json
import csv
import re
from datetime import datetime
from collections import defaultdict, Counter
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import pandas as pd
from pathlib import Path
import argparse
import difflib

# Configuration
REPO_ROOT = Path(__file__).parent
OUTPUT_DIR = REPO_ROOT / "docs_analysis_output"
OUTPUT_DIR.mkdir(exist_ok=True)

# File patterns to include in analysis
DOC_PATTERNS = [
    "*.md",
    "docs/**/*.md",
    "*.rst",
    "docs/**/*.rst"
]

# Key documentation files to track in detail
KEY_FILES = [
    "README.md",
    "CHANGELOG.md",
    "docs/architecture.md",
    "docs/standards.md",
    "docs/testing/README.md"
]

class DocumentationHistoryAnalyzer:
    def __init__(self, repo_path):
        self.repo_path = Path(repo_path)
        self.git_history = []
        self.file_history = defaultdict(list)
        self.file_stats = defaultdict(dict)
        self.commit_data = []
        self.pr_data = []
        
    def run_git_command(self, command):
        """Run a git command and return the output"""
        try:
            result = subprocess.run(
                ["git", "-C", str(self.repo_path)] + command,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError as e:
            print(f"Error running git command {' '.join(command)}: {e}")
            return ""
    
    def get_all_markdown_files(self):
        """Get all markdown files in the repository"""
        cmd = ["ls-files", "*.md"]
        output = self.run_git_command(cmd)
        if output:
            return output.split('\n')
        return []
    
    def extract_file_history(self, file_path):
        """Extract git history for a specific file"""
        cmd = ["log", "--follow", "--pretty=format:%H|%an|%ad|%s", 
               "--date=short", "--stat", "--", file_path]
        output = self.run_git_command(cmd)
        
        if not output:
            return []
        
        commits = []
        current_commit = {}
        lines = output.split('\n')
        
        for line in lines:
            if line and '|' in line and not line.startswith(' '):
                # New commit entry
                if current_commit:
                    commits.append(current_commit)
                
                parts = line.split('|', 3)
                current_commit = {
                    'hash': parts[0],
                    'author': parts[1],
                    'date': parts[2],
                    'message': parts[3] if len(parts) > 3 else "",
                    'file_path': file_path,
                    'files_changed': 0,
                    'insertions': 0,
                    'deletions': 0
                }
            elif line.startswith(' ') and ('changed' in line or 'insertion' in line or 'deletion' in line):
                # Parse file stats
                if current_commit:
                    stats_match = re.search(r'(\d+) files? changed(?:, (\d+) insertions?\(\+\))?(?:, (\d+) deletions?\(-\))?', line)
                    if stats_match:
                        current_commit['files_changed'] = int(stats_match.group(1) or 0)
                        current_commit['insertions'] = int(stats_match.group(2) or 0)
                        current_commit['deletions'] = int(stats_match.group(3) or 0)
        
        if current_commit:
            commits.append(current_commit)
            
        return commits
    
    def extract_all_documentation_history(self):
        """Extract git history for all documentation files"""
        print("Extracting documentation file history...")
        
        # Get all markdown files
        markdown_files = self.get_all_markdown_files()
        print(f"Found {len(markdown_files)} markdown files")
        
        # Extract history for each file
        for file_path in markdown_files:
            print(f"Processing {file_path}...")
            file_commits = self.extract_file_history(file_path)
            self.file_history[file_path] = file_commits
            self.git_history.extend(file_commits)
        
        # Sort by date
        self.git_history.sort(key=lambda x: x['date'])
        
        print(f"Extracted {len(self.git_history)} total documentation commits")
        return self.git_history
    
    def get_file_content_at_commit(self, file_path, commit_hash):
        """Get file content at a specific commit"""
        cmd = ["show", f"{commit_hash}:{file_path}"]
        output = self.run_git_command(cmd)
        return output
    
    def analyze_file_changes(self, file_path, commit1, commit2):
        """Analyze changes between two commits for a file"""
        content1 = self.get_file_content_at_commit(file_path, commit1)
        content2 = self.get_file_content_at_commit(file_path, commit2)
        
        if not content1:
            content1 = ""
        if not content2:
            content2 = ""
            
        diff = list(difflib.unified_diff(
            content1.splitlines(keepends=True),
            content2.splitlines(keepends=True),
            fromfile=f"{file_path}@{commit1[:7]}",
            tofile=f"{file_path}@{commit2[:7]}",
            lineterm=''
        ))
        
        return ''.join(diff)
    
    def identify_major_changes(self, file_path, threshold_lines=50):
        """Identify major changes in a file based on line count"""
        commits = self.file_history.get(file_path, [])
        major_changes = []
        
        for i, commit in enumerate(commits):
            total_changes = commit['insertions'] + commit['deletions']
            if total_changes >= threshold_lines:
                major_changes.append({
                    'commit': commit,
                    'change_type': 'major',
                    'total_changes': total_changes
                })
        
        return major_changes
    
    def generate_timeline_data(self):
        """Generate data for timeline visualization"""
        timeline_data = defaultdict(int)
        
        for commit in self.git_history:
            date = commit['date']
            timeline_data[date] += 1
        
        # Sort by date
        sorted_dates = sorted(timeline_data.items())
        dates = [item[0] for item in sorted_dates]
        counts = [item[1] for item in sorted_dates]
        
        return dates, counts
    
    def create_timeline_visualization(self):
        """Create a timeline visualization of documentation changes"""
        dates, counts = self.generate_timeline_data()
        
        # Convert dates to datetime objects for plotting
        date_objects = [datetime.strptime(d, '%Y-%m-%d') for d in dates]
        
        plt.figure(figsize=(12, 6))
        plt.plot(date_objects, counts, marker='o', linestyle='-')
        plt.title('Documentation Changes Over Time')
        plt.xlabel('Date')
        plt.ylabel('Number of Documentation Commits')
        plt.grid(True)
        
        # Format x-axis
        plt.gca().xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m'))
        plt.gca().xaxis.set_major_locator(mdates.MonthLocator(interval=1))
        plt.xticks(rotation=45)
        plt.tight_layout()
        
        output_path = OUTPUT_DIR / "documentation_timeline.png"
        plt.savefig(output_path)
        plt.close()
        
        return str(output_path)
    
    def create_file_size_evolution(self, file_path):
        """Create visualization of file size evolution"""
        commits = self.file_history.get(file_path, [])
        if not commits:
            return None
            
        # Get file size at each commit
        sizes = []
        dates = []
        
        for commit in commits:
            content = self.get_file_content_at_commit(file_path, commit['hash'])
            size = len(content) if content else 0
            sizes.append(size)
            dates.append(commit['date'])
        
        # Convert dates to datetime objects
        date_objects = [datetime.strptime(d, '%Y-%m-%d') for d in dates]
        
        plt.figure(figsize=(12, 6))
        plt.plot(date_objects, sizes, marker='o', linestyle='-')
        plt.title(f'File Size Evolution: {file_path}')
        plt.xlabel('Date')
        plt.ylabel('File Size (characters)')
        plt.grid(True)
        
        # Format x-axis
        plt.gca().xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m'))
        plt.xticks(rotation=45)
        plt.tight_layout()
        
        # Create safe filename
        safe_filename = file_path.replace('/', '_').replace('\\', '_')
        output_path = OUTPUT_DIR / f"file_size_{safe_filename}.png"
        plt.savefig(output_path)
        plt.close()
        
        return str(output_path)
    
    def export_to_json(self):
        """Export data to JSON format"""
        data = {
            'file_history': dict(self.file_history),
            'git_history': self.git_history
        }
        
        output_path = OUTPUT_DIR / "documentation_history.json"
        with open(output_path, 'w') as f:
            json.dump(data, f, indent=2)
        
        return str(output_path)
    
    def export_to_csv(self):
        """Export data to CSV format"""
        output_path = OUTPUT_DIR / "documentation_history.csv"
        
        with open(output_path, 'w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow([
                'Hash', 'Author', 'Date', 'Message', 'File Path',
                'Files Changed', 'Insertions', 'Deletions'
            ])
            
            for commit in self.git_history:
                writer.writerow([
                    commit['hash'],
                    commit['author'],
                    commit['date'],
                    commit['message'],
                    commit['file_path'],
                    commit['files_changed'],
                    commit['insertions'],
                    commit['deletions']
                ])
        
        return str(output_path)
    
    def generate_report(self):
        """Generate a comprehensive analysis report"""
        report_path = OUTPUT_DIR / "documentation_analysis_report.md"
        
        with open(report_path, 'w') as f:
            f.write("# Documentation History Analysis Report\n\n")
            f.write(f"Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            
            # Summary statistics
            f.write("## Summary Statistics\n\n")
            f.write(f"- Total documentation files: {len(self.file_history)}\n")
            f.write(f"- Total documentation commits: {len(self.git_history)}\n")
            
            # Most active authors
            authors = Counter(commit['author'] for commit in self.git_history)
            f.write("\n### Most Active Authors\n\n")
            for author, count in authors.most_common(10):
                f.write(f"- {author}: {count} commits\n")
            
            # Most changed files
            file_changes = Counter(commit['file_path'] for commit in self.git_history)
            f.write("\n### Most Changed Files\n\n")
            for file_path, count in file_changes.most_common(10):
                f.write(f"- {file_path}: {count} commits\n")
            
            # Major changes
            f.write("\n## Major Changes\n\n")
            for file_path in KEY_FILES:
                if file_path in self.file_history:
                    major_changes = self.identify_major_changes(file_path)
                    if major_changes:
                        f.write(f"### {file_path}\n\n")
                        for change in major_changes:
                            commit = change['commit']
                            f.write(f"- **{commit['date']}** ({commit['hash'][:7]}): ")
                            f.write(f"{commit['message']} ")
                            f.write(f"({change['total_changes']} lines changed)\n")
        
        return str(report_path)
    
    def run_full_analysis(self):
        """Run the complete analysis pipeline"""
        print("Starting comprehensive documentation history analysis...")
        
        # Extract history
        self.extract_all_documentation_history()
        
        # Create visualizations
        print("Creating timeline visualization...")
        timeline_path = self.create_timeline_visualization()
        print(f"Timeline visualization saved to: {timeline_path}")
        
        # Create file size evolution for key files
        for file_path in KEY_FILES:
            if file_path in self.file_history:
                print(f"Creating file size evolution for {file_path}...")
                size_path = self.create_file_size_evolution(file_path)
                if size_path:
                    print(f"File size visualization saved to: {size_path}")
        
        # Export data
        print("Exporting data to JSON...")
        json_path = self.export_to_json()
        print(f"Data exported to: {json_path}")
        
        print("Exporting data to CSV...")
        csv_path = self.export_to_csv()
        print(f"Data exported to: {csv_path}")
        
        # Generate report
        print("Generating analysis report...")
        report_path = self.generate_report()
        print(f"Report generated at: {report_path}")
        
        print("Analysis complete!")
        return {
            'timeline': timeline_path,
            'json': json_path,
            'csv': csv_path,
            'report': report_path
        }


def main():
    parser = argparse.ArgumentParser(description='Analyze documentation history in a Git repository')
    parser.add_argument('--repo', default=str(REPO_ROOT), help='Path to the repository')
    
    args = parser.parse_args()
    
    analyzer = DocumentationHistoryAnalyzer(args.repo)
    results = analyzer.run_full_analysis()
    
    print("\nAnalysis Results:")
    for key, path in results.items():
        print(f"- {key}: {path}")


if __name__ == "__main__":
    main()