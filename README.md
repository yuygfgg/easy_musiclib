## EASY MUSIC LIBRARY

A music library server, including server, cli client, and webui. Supports muitiple artists per track and album.

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
