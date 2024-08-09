## EASY MUSIC LIBRARY

A music library server, including server, cli client, and webui. Supports muitiple artists per track and album.

### Screen Shots




https://github.com/yuygfgg/easy_musiclib/assets/140488233/9d9ad2b4-c3d5-4b4a-9faa-0ac6fe2a8cd8


<img width="1102" alt="截屏2024-08-09 20 28 06" src="https://github.com/user-attachments/assets/135cb234-4de0-47ea-bd65-ae1dd5fd1eeb">
<img width="1104" alt="截屏2024-08-09 20 28 24" src="https://github.com/user-attachments/assets/553fc817-53ed-44c1-97f4-641df6e341e5">
<img width="1270" alt="截屏2024-08-09 20 31 02" src="https://github.com/user-attachments/assets/4e1d30d2-e49f-4bef-aa4c-15bb53d257fa">
<img width="1273" alt="截屏2024-08-09 20 33 30" src="https://github.com/user-attachments/assets/ba397058-4579-46c2-be78-c5fbd0626bd6">
<img width="1270" alt="截屏2024-08-09 20 33 55" src="https://github.com/user-attachments/assets/ffa418fa-6450-4ef0-b2ea-e60c93eef61f">
<img width="1263" alt="截屏2024-08-09 20 36 02" src="https://github.com/user-attachments/assets/d1b853ca-3e24-41cd-ae67-653e1bd259ab">
<img width="1266" alt="截屏2024-08-09 20 39 39" src="https://github.com/user-attachments/assets/7dd60dc4-29e1-4ef5-ba81-fb2637d1f1c3">


### Schedual

- mobile app : ~ 2025

The project will continue unless I find another music library server that handles muitiple artist  and muiti-disc albums correctly.

Webui is currently in **Chinese** only.

### Usage

#### Server Setup
1. Clone the repository
2. Install all necessary python packages ``` pypy3 -m pip install -r requirements.txt ```
3. Run server ```pypy3 server.py ```

#### Scan Directory
1. install cURL
2. run command ``` curl "http://server_address/scan?directory=path_to_music" ```

#### Webui
1. webui is avaliable at http://server_address:5010/

#### API
1. APIs are at http://server_address:5010/api/


### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.
