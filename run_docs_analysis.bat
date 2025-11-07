@echo off
REM Batch script to run documentation history analysis
REM This script runs both the basic and cross-reference analyzers

echo Starting Documentation History Analysis for Coldvox Repository
echo ==========================================================
echo.

REM Check if Python is installed
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: Python is not installed or not in PATH
    echo Please install Python and try again
    pause
    exit /b 1
)

REM Install required packages
echo Installing required Python packages...
pip install -r docs_analysis_requirements.txt
if %errorlevel% neq 0 (
    echo Error: Failed to install required packages
    pause
    exit /b 1
)

echo.
echo Running basic documentation history analysis...
python docs_history_analyzer.py --repo .
if %errorlevel% neq 0 (
    echo Error: Basic documentation history analysis failed
    pause
    exit /b 1
)

echo.
echo Running cross-reference documentation analysis...
python docs_cross_reference_analyzer.py --repo .
if %errorlevel% neq 0 (
    echo Error: Cross-reference documentation analysis failed
    pause
    exit /b 1
)

echo.
echo ==========================================================
echo Documentation analysis complete!
echo.
echo Results have been saved to:
echo - docs_analysis_output\ (basic analysis)
echo - docs_cross_reference_output\ (cross-reference analysis)
echo.
echo Please check the generated reports and visualizations.
echo ==========================================================
pause