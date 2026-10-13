@echo off
chcp 65001 >nul
echo ========================================================
echo   TBK CORE - TỰ ĐỘNG ĐẨY CODE LÊN GITHUB
echo   Repo: https://github.com/VUONGBAO512/TBK-CORE
echo ========================================================
echo.

cd /d "d:\HOCC++\pc-doctor"

echo [1/3] Đang kiểm tra và thêm file thay đổi (git add)...
git add .

echo.
set /p commit_msg="Nhập nội dung commit (bấm Enter để dùng mặc định 'update code'): "
if "%commit_msg%"=="" set commit_msg=update code

echo.
echo [2/3] Đang tạo commit: "%commit_msg%"...
git commit -m "%commit_msg%"

echo.
echo [3/3] Đang đẩy code lên GitHub (git push origin main)...
git push origin main

echo.
if %errorlevel% equ 0 (
    echo ========================================================
    echo   [THÀNH CÔNG] Code đã được đẩy lên GitHub!
    echo   Xem tại: https://github.com/VUONGBAO512/TBK-CORE
    echo ========================================================
) else (
    echo ========================================================
    echo   [LỖI] Không thể đẩy code. Vui lòng kiểm tra lại mạng hoặc token!
    echo ========================================================
)

echo.
pause
