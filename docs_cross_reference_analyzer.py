#!/usr/bin/env python3
"""
Advanced Documentation Cross-Reference Analyzer for Coldvox Repository

This script extends the basic documentation history analysis by:
1. Cross-referencing documentation changes with pull requests
2. Correlating documentation updates with code changes
3. Mapping documentation changes to release versions
4. Identifying patterns between codebase changes and documentation updates
"""

import os
import subprocess
import json
import re
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import pandas as pd
from pathlib import Path
import argparse
import difflib
import requests
import time

# Configuration
REPO_ROOT = Path(__file__).parent
OUTPUT_DIR = REPO_ROOT / "docs_cross_reference_output"
OUTPUT_DIR.mkdir(exist_ok=True)

# GitHub API configuration (if needed)
# Note: For private repos or rate-limited access, you may need to provide a token
GITHUB_API_URL = "https://api.github.com"
GITHUB_TOKEN = os.environ.get("GITHUB_TOKEN", None)

class DocumentationCrossReferenceAnalyzer:
    def __init__(self, repo_path, repo_owner=None, repo_name=None, since=None):
        self.repo_path = Path(repo_path)
        self.repo_owner = repo_owner
        self.repo_name = repo_name
        self.since = since
        self.git_history = []
        self.file_history = defaultdict(list)
        self.pr_data = []
        self.release_data = []
        self.code_changes = defaultdict(list)
        self.doc_code_correlations = []
        
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
    
    def get_all_rust_files(self):
        """Get all Rust source files in the repository"""
        cmd = ["ls-files", "*.rs"]
        output = self.run_git_command(cmd)
        if output:
            return output.split('\n')
        return []
    
    def extract_file_history(self, file_path):
        """Extract git history for a specific file"""
        cmd = ["log", "--follow", "--pretty=format:%H|%an|%ad|%s", 
               "--date=short", "--stat"]
        if self.since:
            cmd.extend(["--since", self.since])
        cmd.extend(["--", file_path])
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
    
    def extract_all_code_history(self):
        """Extract git history for all code files"""
        print("Extracting code file history...")
        
        # Get all Rust files
        rust_files = self.get_all_rust_files()
        print(f"Found {len(rust_files)} Rust files")
        
        # Extract history for each file
        for file_path in rust_files:
            file_commits = self.extract_file_history(file_path)
            self.code_changes[file_path] = file_commits
        
        print(f"Extracted code history for {len(self.code_changes)} files")
        return self.code_changes
    
    def extract_release_history(self):
        """Extract release history from git tags"""
        print("Extracting release history...")
        
        # Get all tags
        cmd = ["tag", "-l", "--sort=-version:refname"]
        output = self.run_git_command(cmd)
        
        if not output:
            return []
        
        tags = output.split('\n')
        releases = []
        
        for tag in tags:
            if not tag:
                continue
                
            # Get tag date
            cmd = ["log", "-1", "--format=%ad", "--date=short", tag]
            date = self.run_git_command(cmd)
            
            # Get tag message
            cmd = ["tag", "-l", "--format=%(contents)", tag]
            message = self.run_git_command(cmd)
            
            releases.append({
                'tag': tag,
                'date': date,
                'message': message
            })
        
        self.release_data = sorted(releases, key=lambda x: x['date'])
        print(f"Extracted {len(self.release_data)} releases")
        return self.release_data
    
    def extract_pr_data_from_commit_messages(self):
        """Extract PR information from commit messages"""
        print("Extracting PR data from commit messages...")
        
        pr_pattern = re.compile(r'#(\d+)')
        pr_data = []
        
        for commit in self.git_history:
            matches = pr_pattern.findall(commit['message'])
            for pr_num in matches:
                pr_data.append({
                    'pr_number': int(pr_num),
                    'commit_hash': commit['hash'],
                    'commit_date': commit['date'],
                    'commit_message': commit['message'],
                    'file_path': commit['file_path']
                })
        
        self.pr_data = pr_data
        print(f"Found {len(pr_data)} PR references in documentation commits")
        
        if GITHUB_TOKEN and self.repo_owner and self.repo_name:
            self.enrich_pr_data_with_github()
            
        return self.pr_data

    def enrich_pr_data_with_github(self):
        """Enrich PR data with information from GitHub API"""
        print("Enriching PR data with GitHub API...")
        enriched_pr_data = []
        
        headers = {
            "Authorization": f"token {GITHUB_TOKEN}",
            "Accept": "application/vnd.github.v3+json"
        }
        
        # Get unique PR numbers
        pr_numbers = sorted(list(set(pr['pr_number'] for pr in self.pr_data)))
        
        for pr_num in pr_numbers:
            url = f"{GITHUB_API_URL}/repos/{self.repo_owner}/{self.repo_name}/pulls/{pr_num}"
            try:
                response = requests.get(url, headers=headers)
                response.raise_for_status()
                
                pr_details = response.json()
                
                # Find all commits associated with this PR
                for pr_commit_data in self.pr_data:
                    if pr_commit_data['pr_number'] == pr_num:
                        enriched_pr_data.append({
                            **pr_commit_data,
                            'pr_title': pr_details.get('title', ''),
                            'pr_author': pr_details.get('user', {}).get('login', ''),
                            'pr_state': pr_details.get('state', ''),
                            'pr_url': pr_details.get('html_url', '')
                        })
                
                print(f"Enriched data for PR #{pr_num}")
                
            except requests.exceptions.RequestException as e:
                print(f"Error fetching data for PR #{pr_num}: {e}")
                # Add without enrichment if API fails
                for pr_commit_data in self.pr_data:
                    if pr_commit_data['pr_number'] == pr_num:
                        enriched_pr_data.append(pr_commit_data)

            # Rate limit handling
            time.sleep(1)

        self.pr_data = enriched_pr_data
        print("Finished enriching PR data.")
    
    def correlate_doc_code_changes(self, days_window=7):
        """Find correlations between documentation and code changes"""
        print("Correlating documentation and code changes...")
        
        correlations = []
        
        # Create a date-indexed map of code changes
        code_changes_by_date = defaultdict(list)
        for file_path, commits in self.code_changes.items():
            for commit in commits:
                code_changes_by_date[commit['date']].append({
                    'file_path': file_path,
                    'commit_hash': commit['hash'],
                    'message': commit['message']
                })
        
        # For each documentation commit, find nearby code changes
        for doc_commit in self.git_history:
            doc_date = datetime.strptime(doc_commit['date'], '%Y-%m-%d')
            
            # Look for code changes within the window
            for i in range(-days_window, days_window + 1):
                check_date = (doc_date + timedelta(days=i)).strftime('%Y-%m-%d')
                if check_date in code_changes_by_date:
                    for code_change in code_changes_by_date[check_date]:
                        correlations.append({
                            'doc_commit': doc_commit,
                            'code_change': code_change,
                            'days_diff': i,
                            'date': doc_commit['date']
                        })
        
        self.doc_code_correlations = correlations
        print(f"Found {len(correlations)} documentation-code correlations")
        return correlations
    
    def create_doc_code_correlation_chart(self):
        """Create a chart showing documentation-code correlations over time"""
        if not self.doc_code_correlations:
            return None
        
        # Group correlations by date
        correlations_by_date = defaultdict(int)
        for corr in self.doc_code_correlations:
            correlations_by_date[corr['date']] += 1
        
        # Sort by date
        sorted_dates = sorted(correlations_by_date.items())
        dates = [item[0] for item in sorted_dates]
        counts = [item[1] for item in sorted_dates]
        
        # Convert dates to datetime objects for plotting
        date_objects = [datetime.strptime(d, '%Y-%m-%d') for d in dates]
        
        plt.figure(figsize=(12, 6))
        plt.plot(date_objects, counts, marker='o', linestyle='-')
        plt.title('Documentation-Code Correlations Over Time')
        plt.xlabel('Date')
        plt.ylabel('Number of Correlations')
        plt.grid(True)
        
        # Format x-axis
        plt.gca().xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m'))
        plt.gca().xaxis.set_major_locator(mdates.MonthLocator(interval=1))
        plt.xticks(rotation=45)
        plt.tight_layout()
        
        output_path = OUTPUT_DIR / "doc_code_correlations.png"
        plt.savefig(output_path)
        plt.close()
        
        return str(output_path)
    
    def create_release_doc_correlation_chart(self):
        """Create a chart showing documentation activity around releases"""
        if not self.release_data or not self.git_history:
            return None
        
        # Group documentation commits by date
        doc_commits_by_date = defaultdict(int)
        for commit in self.git_history:
            doc_commits_by_date[commit['date']] += 1
        
        # Create a timeline with releases marked
        plt.figure(figsize=(14, 8))
        
        # Plot documentation activity
        sorted_dates = sorted(doc_commits_by_date.items())
        dates = [item[0] for item in sorted_dates]
        counts = [item[1] for item in sorted_dates]
        
        date_objects = [datetime.strptime(d, '%Y-%m-%d') for d in dates]
        plt.plot(date_objects, counts, marker='o', linestyle='-', alpha=0.7, label='Documentation Commits')
        
        # Mark releases
        for release in self.release_data:
            release_date = datetime.strptime(release['date'], '%Y-%m-%d')
            plt.axvline(x=release_date, color='r', linestyle='--', alpha=0.7)
            plt.text(release_date, max(counts) * 0.9, release['tag'], rotation=90, 
                    verticalalignment='top', color='r')
        
        plt.title('Documentation Activity Around Releases')
        plt.xlabel('Date')
        plt.ylabel('Number of Documentation Commits')
        plt.grid(True)
        plt.legend()
        
        # Format x-axis
        plt.gca().xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m'))
        plt.gca().xaxis.set_major_locator(mdates.MonthLocator(interval=1))
        plt.xticks(rotation=45)
        plt.tight_layout()
        
        output_path = OUTPUT_DIR / "release_doc_correlations.png"
        plt.savefig(output_path)
        plt.close()
        
        return str(output_path)
    
    def export_to_json(self):
        """Export data to JSON format"""
        data = {
            'file_history': dict(self.file_history),
            'git_history': self.git_history,
            'code_changes': dict(self.code_changes),
            'pr_data': self.pr_data,
            'release_data': self.release_data,
            'doc_code_correlations': self.doc_code_correlations
        }
        
        output_path = OUTPUT_DIR / "documentation_cross_reference.json"
        with open(output_path, 'w') as f:
            json.dump(data, f, indent=2)
        
        return str(output_path)
    
    def generate_cross_reference_report(self):
        """Generate a comprehensive cross-reference analysis report"""
        report_path = OUTPUT_DIR / "documentation_cross_reference_report.md"
        
        with open(report_path, 'w') as f:
            f.write("# Documentation Cross-Reference Analysis Report\n\n")
            f.write(f"Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            
            # Summary statistics
            f.write("## Summary Statistics\n\n")
            f.write(f"- Total documentation files: {len(self.file_history)}\n")
            f.write(f"- Total documentation commits: {len(self.git_history)}\n")
            f.write(f"- Total code files: {len(self.code_changes)}\n")
            f.write(f"- Total releases: {len(self.release_data)}\n")
            f.write(f"- Documentation-code correlations: {len(self.doc_code_correlations)}\n")
            
            # PR analysis
            if self.pr_data:
                f.write("\n## Pull Request Analysis\n\n")
                pr_counts = Counter(pr['pr_number'] for pr in self.pr_data)
                f.write(f"- Total PRs referenced in documentation: {len(pr_counts)}\n")
                f.write("\n### Most Referenced PRs\n\n")
                # Create a map of PR number to details for easy lookup
                pr_details_map = {}
                for pr in self.pr_data:
                    if pr['pr_number'] not in pr_details_map:
                        pr_details_map[pr['pr_number']] = {
                            'title': pr.get('pr_title', ''),
                            'author': pr.get('pr_author', ''),
                            'state': pr.get('pr_state', '')
                        }

                for pr_num, count in pr_counts.most_common(10):
                    details = pr_details_map.get(pr_num, {})
                    title = details.get('title', '[Title not fetched]')
                    author = details.get('author', 'N/A')
                    f.write(f"- PR #{pr_num}: {count} documentation commits - *{title}* (by @{author})\n")
            
            # Release analysis
            if self.release_data:
                f.write("\n## Release Analysis\n\n")
                f.write("### Release Timeline\n\n")
                for release in self.release_data:
                    f.write(f"- **{release['tag']}** ({release['date']}): {release['message'][:50]}...\n")
            
            # Correlation analysis
            if self.doc_code_correlations:
                f.write("\n## Documentation-Code Correlation Analysis\n\n")
                
                # Correlations by days difference
                days_diff_counts = Counter(corr['days_diff'] for corr in self.doc_code_correlations)
                f.write("### Correlations by Days Difference\n\n")
                for days_diff, count in sorted(days_diff_counts.items()):
                    f.write(f"- {days_diff} days: {count} correlations\n")
                
                # Most correlated files
                doc_files = Counter(corr['doc_commit']['file_path'] for corr in self.doc_code_correlations)
                f.write("\n### Most Correlated Documentation Files\n\n")
                for file_path, count in doc_files.most_common(10):
                    f.write(f"- {file_path}: {count} correlations\n")
                
                code_files = Counter(corr['code_change']['file_path'] for corr in self.doc_code_correlations)
                f.write("\n### Most Correlated Code Files\n\n")
                for file_path, count in code_files.most_common(10):
                    f.write(f"- {file_path}: {count} correlations\n")
        
        return str(report_path)
    
    def run_full_analysis(self):
        """Run the complete cross-reference analysis pipeline"""
        print("Starting comprehensive documentation cross-reference analysis...")
        
        # Extract histories
        self.extract_all_documentation_history()
        self.extract_all_code_history()
        self.extract_release_history()
        self.extract_pr_data_from_commit_messages()
        
        # Correlate changes
        self.correlate_doc_code_changes()
        
        # Create visualizations
        print("Creating documentation-code correlation chart...")
        corr_chart_path = self.create_doc_code_correlation_chart()
        if corr_chart_path:
            print(f"Correlation chart saved to: {corr_chart_path}")
        
        print("Creating release-documentation correlation chart...")
        release_chart_path = self.create_release_doc_correlation_chart()
        if release_chart_path:
            print(f"Release chart saved to: {release_chart_path}")
        
        # Export data
        print("Exporting data to JSON...")
        json_path = self.export_to_json()
        print(f"Data exported to: {json_path}")
        
        # Generate report
        print("Generating cross-reference analysis report...")
        report_path = self.generate_cross_reference_report()
        print(f"Report generated at: {report_path}")
        
        print("Cross-reference analysis complete!")
        return {
            'correlation_chart': corr_chart_path,
            'release_chart': release_chart_path,
            'json': json_path,
            'report': report_path
        }


def main():
    parser = argparse.ArgumentParser(description='Analyze documentation cross-references in a Git repository')
    parser.add_argument('--repo', default=str(REPO_ROOT), help='Path to the repository')
    parser.add_argument('--owner', help='Repository owner (for GitHub API)')
    parser.add_argument('--name', help='Repository name (for GitHub API)')
    parser.add_argument('--since', help='Start date for git history analysis (e.g., YYYY-MM-DD)')
    
    args = parser.parse_args()
    
    analyzer = DocumentationCrossReferenceAnalyzer(
        args.repo, args.owner, args.name, args.since
    )
    results = analyzer.run_full_analysis()
    
    print("\nAnalysis Results:")
    for key, path in results.items():
        if path:
            print(f"- {key}: {path}")


if __name__ == "__main__":
    main()