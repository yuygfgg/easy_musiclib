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

<img width="1857" alt="截屏2024-06-29 22 27 19" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/09ebb3ea-d49d-4bc9-934d-8b347642e26c">
<img width="1847" alt="截屏2024-06-29 22 27 43" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/767777d9-7275-4c30-b424-b28121ae8d0f">
<img width="1858" alt="截屏2024-06-29 22 28 26" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/f59931d4-649f-4658-acfc-b5cf2a928fc7">
<img width="1866" alt="截屏2024-06-29 22 28 55" src="https://github.com/yuygfgg/easy_musiclib/assets/140488233/4f53f0b5-ac7f-436e-8d20-b212cd6fc65d">

### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.
