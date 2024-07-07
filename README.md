## EASY MUSIC LIBRARY

A music library server, including server, cli client, and webui. Supports muitiple artists per track and album.

### Screen Shots




https://github.com/yuygfgg/easy_musiclib/assets/140488233/9d9ad2b4-c3d5-4b4a-9faa-0ac6fe2a8cd8




<img width="1863" alt="截屏2024-07-01 21 07 41" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/77a8b735-4dba-45d3-83a3-99a6310f1cb2">
<img width="1865" alt="截屏2024-07-01 21 09 07" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/934f6f9f-5e67-41f0-85bb-b0d2caf4191f">
<img width="1860" alt="截屏2024-07-01 21 10 09" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/7350dadd-312d-432d-8bc6-fa5f98749311">

### Schedual

- add lyrics support : before 2024.07.10
- mobile app : ~ 2025

The project will continue unless I find another music library server that handles muitiple artist  and muiti-disc albums correctly.

Webui is currently in **Chinese** only.

### Usage

#### Server Setup
1. Clone the repository
2. Install all necessary python packages ``` pip install -r requirements.txt ```
3. install ``` mpv ```
4. Run api.py ```python api.py ```
5. modify ``` base_url ``` in ``` client.py ```
6. run cli client ```python client.py```
7. run webui server in ``` webui ``` folder:  ``` node server.js ```

#### Scan Directory
1. run cli client ```python client.py```
2. run command ``` scan_directory /path/to/music ```


### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.
