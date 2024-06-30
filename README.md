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
3. install http-server ``` npm install -g http-server ```
4. install ``` mpv ```
5. Run api.py ```python api.py ```
6. modify ``` base_url ``` in ``` client.py ```
7. run cli client ```python client.py```
8. run http-server ``` http-server ```

#### Scan Directory
1. run cli client ```python client.py```
2. run command ``` scan_directory /path/to/music ```


### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.
