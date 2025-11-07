# Documentation History Analysis Tools

This directory contains tools for analyzing the evolution of documentation in the Coldvox Git repository. These tools provide comprehensive insights into how documentation has changed over time, correlations with code changes, and patterns around releases.

## Tools Overview

### 1. Basic Documentation History Analyzer (`docs_history_analyzer.py`)

This script performs a comprehensive analysis of documentation history in the Git repository, including:

- **Git History Extraction**: Parses the complete git commit history focusing on markdown (.md) files
- **Data Processing Pipeline**: Structures the data into a queryable format (JSON/CSV)
- **Change Analysis**: Implements diff analysis between major documentation versions
- **Visualization Deliverables**: Creates timeline visualizations and file-specific change histories
- **Reporting**: Compiles findings into an analysis report with quantitative and qualitative observations

### 2. Documentation Cross-Reference Analyzer (`docs_cross_reference_analyzer.py`)

This advanced script extends the basic analysis by:

- Cross-referencing documentation changes with pull requests
- Correlating documentation updates with code changes
- Mapping documentation changes to release versions
- Identifying patterns between codebase changes and documentation updates

## Installation and Setup

### Prerequisites

- Python 3.6 or higher
- Git installed and accessible from command line
- (Optional) GitHub token for enhanced PR analysis (set as `GITHUB_TOKEN` environment variable)

### Installing Dependencies

The required Python packages are listed in `docs_analysis_requirements.txt`. Install them with:

```bash
pip install -r docs_analysis_requirements.txt
```

## Usage

### Quick Start

For a complete analysis with both tools, simply run the batch script:

```bash
run_docs_analysis.bat
```

This will:
1. Install required dependencies
2. Run the basic documentation history analysis
3. Run the cross-reference analysis
4. Generate all reports and visualizations

### Running Individual Tools

#### Basic Documentation History Analyzer

```bash
python docs_history_analyzer.py --repo .
```

Optional parameters:
- `--repo`: Path to the repository (default: current directory)
- `--output`: Output directory (default: `docs_analysis_output`)

#### Documentation Cross-Reference Analyzer

```bash
python docs_cross_reference_analyzer.py --repo .
```

Optional parameters:
- `--repo`: Path to the repository (default: current directory)
- `--owner`: Repository owner (for GitHub API)
- `--name`: Repository name (for GitHub API)

## Output and Results

### Basic Analysis Output

The basic analyzer creates a `docs_analysis_output` directory with:

- `documentation_history.json`: Raw data extracted from git history
- `documentation_history.csv`: Tabular data for spreadsheet analysis
- `documentation_timeline.png`: Timeline visualization of documentation changes
- `file_size_evolution.png`: Chart showing how documentation files have grown over time
- `documentation_analysis_report.md`: Comprehensive analysis report

### Cross-Reference Analysis Output

The cross-reference analyzer creates a `docs_cross_reference_output` directory with:

- `documentation_cross_reference.json`: Raw data including correlations
- `doc_code_correlations.png`: Chart showing documentation-code correlations over time
- `release_doc_correlations.png`: Chart showing documentation activity around releases
- `documentation_cross_reference_report.md`: Comprehensive cross-reference analysis report

## Interpreting the Results

### Timeline Visualizations

The timeline charts show when documentation changes occurred, with peaks indicating periods of high documentation activity. Look for patterns such as:

- Regular documentation updates (good maintenance practice)
- Clusters of changes around specific dates (possible documentation overhauls)
- Gaps in documentation updates (potential areas for improvement)

### File Size Evolution

The file size evolution chart shows how individual documentation files have grown over time. Look for:

- Rapid growth (possible addition of new sections)
- Stable size (mature documentation)
- Decreases (possible refactoring or simplification)

### Documentation-Code Correlations

The correlation charts show how documentation changes relate to code changes and releases. Look for:

- High correlation (documentation kept in sync with code)
- Low correlation (potential documentation debt)
- Patterns around releases (documentation prepared for releases)

### Analysis Reports

The markdown reports provide detailed insights including:

- Quantitative metrics (change frequency, file growth)
- Qualitative observations (style shifts, content focus changes)
- Key inflection points in documentation history
- Most active documentation files
- Correlations between documentation and code changes

## Customization and Extension

### Adding New Analysis

Both analyzers are designed to be extensible. To add new analysis:

1. Add new methods to the appropriate analyzer class
2. Call the new methods from the `run_full_analysis` method
3. Update the report generation to include new findings

### Changing Visualization Styles

The visualizations use Matplotlib. To customize:

1. Modify the plotting functions in the analyzer classes
2. Adjust colors, markers, and other styling elements
3. Change chart types or add new visualizations

### Extending Data Sources

To incorporate additional data sources:

1. Add new extraction methods to the analyzer classes
2. Update the data processing pipeline to handle new data
3. Modify the export functions to include new data

## Troubleshooting

### Common Issues

1. **Git command not found**: Ensure Git is installed and in your PATH
2. **Permission errors**: Make sure you have read access to the repository
3. **Missing dependencies**: Run `pip install -r docs_analysis_requirements.txt`
4. **Memory issues with large repositories**: Consider limiting the analysis to specific files or time periods

### Performance Considerations

For very large repositories:

- Consider limiting the analysis to specific directories or file patterns
- Increase the time window for correlation analysis to reduce processing time
- Use the `--max-count` parameter in git commands to limit the number of commits analyzed

## Contributing

To contribute improvements to these tools:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

These tools are part of the Coldvox project and follow the same license terms.