## EASY MUSIC LIBRARY

A music library server, including server and cli client. Supports muitiple artists per track and album.

### Schedual

- add lyrics support : before 2024.07.10
- mobile app : ~ 2025

The project will continue unless I find another music library server that handles muitiple artist  and muiti-disc albums correctly.

Webui is currently in **Chinese** only.

### Usage

#### Server Setup
1. Clone the repository
2. In the folder, modify ``` apiBaseUrl ``` in every ``` .html ``` file to your own
3. Install all necessary python packages ``` pip install -r requirements.txt ```
4. install http-server ``` npm install -g http-server ```
5. install ``` mpv ```
6. Run api.py ```python api.py ```
7. modify ``` base_url ``` in ``` client.py ```
8. run cli client ```python client.py```
9. run http-server ``` http-server ```

#### Scan Directory
1. run cli client ```python client.py```
2. run command ``` scan_directory /path/to/music ```


### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.