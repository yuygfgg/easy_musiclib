
        let apiBaseUrl = 'http://127.0.0.1:5010';
        const urlParams = new URLSearchParams(window.location.search);
        const artistUUID = urlParams.get('uuid');
        apiBaseUrl = urlParams.get('apibaseUrl');

        if (!artistUUID) {
            document.body.innerHTML = '<h1>缺少艺术家UUID参数</h1>';
            throw new Error('缺少艺术家UUID参数');
        }

        let currentIndex = 0;
        let songs = [];
        let currentSong = null;

        function navigateTorelation() {
            window.parent.postMessage({ action: 'navigateTo', url: `relation.html?uuid=${artistUUID}` }, '*');
        }

        async function fetchArtistData() {
            try {
                const response = await fetch(`${apiBaseUrl}/show_artist/${artistUUID}`);
                if (response.ok) {
                    const artistData = await response.json();
                    console.log('Fetched artist data:', artistData);
                    songs = artistData.songs;
                    displayArtistData(artistData);
                } else {
                    console.error('Failed to fetch artist data:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function fetchFile(filePath) {
            try {
                const response = await fetch(`${apiBaseUrl}/getfile?file_path=${encodeURIComponent(filePath)}`);
                if (response.ok) {
                    return response.blob();
                } else {
                    console.error('Failed to fetch file:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
            return null;
        }

        async function displayArtistData(artist) {
            const artistArtBlob = await fetchFile(artist.artist_art_path);
            if (artistArtBlob) {
                const artistArtUrl = URL.createObjectURL(artistArtBlob);
                document.getElementById('artist-art').src = artistArtUrl;
            } else {
                document.getElementById('artist-art').src = 'default_artist.png';
            }

            document.getElementById('artist-name').textContent = artist.name;

            const artistLike = document.getElementById('artist-like');
            artistLike.className = artist.is_liked ? 'artist-like text-danger' : 'artist-like';
            artistLike.innerHTML = artist.is_liked ? '&#9829;' : '&#9825;';
            artistLike.onclick = toggleArtistLike;

            const albumList = document.getElementById('album-list');
            albumList.innerHTML = '';
            artist.albums.forEach(async album => {
                const albumItem = document.createElement('div');
                albumItem.className = 'col-md-3 col-sm-4 col-6 album-item';

                const card = document.createElement('div');
                card.className = 'card bg-dark text-white';
                card.style.cursor = 'pointer';

                card.onclick = () => {
                    window.parent.postMessage({ action: 'navigateTo', url: `album.html?uuid=${album.uuid}` }, '*'); // 跳转到专辑详情页
                };

                const albumArt = document.createElement('img');
                albumArt.className = 'album-art card-img-top';
                const albumArtBlob = await fetchFile(album.album_art_path);
                if (albumArtBlob) {
                    const albumArtUrl = URL.createObjectURL(albumArtBlob);
                    albumArt.src = albumArtUrl;
                } else {
                    albumArt.src = 'default_album.png';
                }
                albumArt.alt = album.name;

                const cardBody = document.createElement('div');
                cardBody.className = 'card-body';

                const albumName = document.createElement('h5');
                albumName.className = 'card-title';
                albumName.textContent = album.name;

                const albumYear = document.createElement('p');
                albumYear.className = 'card-text';
                albumYear.textContent = album.year;

                cardBody.appendChild(albumName);
                cardBody.appendChild(albumYear);
                card.appendChild(albumArt);
                card.appendChild(cardBody);
                albumItem.appendChild(card);
                albumList.appendChild(albumItem);
            });

            const songList = document.getElementById('song-list');
            songList.innerHTML = '';
            songs.sort((a, b) => {
                if (a.album.name !== b.album.name) {
                    return a.album.name.localeCompare(b.album.name);
                } else if (a.disc_number !== b.disc_number) {
                    return a.disc_number - b.disc_number;
                } else {
                    return a.track_number - b.track_number;
                }
            });

            songs.forEach((song, index) => {
                const songItem = document.createElement('div');
                songItem.className = 'song-item';
                songItem.id = `song-${song.uuid}`;
                songItem.onclick = (event) => {
                    if (event.target.tagName.toLowerCase() !== 'span' && event.target.tagName.toLowerCase() !== 'a') {
                        fetchAndPlaySong(index);
                    }
                };

                const songInfo = document.createElement('div');
                songInfo.className = 'song-info';

                const songNumber = document.createElement('div');
                songNumber.className = 'number';
                songNumber.textContent = index + 1;

                const songArt = document.createElement('img');
                songArt.className = 'song-art';
                fetchFile(song.song_art_path).then(songArtBlob => {
                    if (songArtBlob) {
                        const songArtUrl = URL.createObjectURL(songArtBlob);
                        songArt.src = songArtUrl;
                    } else {
                        songArt.src = 'default_cover.png';
                    }
                    songArt.alt = song.name;
                });

                const songDetails = document.createElement('div');
                songDetails.className = 'details';

                const songTitle = document.createElement('div');
                songTitle.className = 'title';
                songTitle.textContent = song.name;

                const songArtist = document.createElement('div');
                songArtist.className = 'artist';
                song.artists.forEach(artist => {
                    const artistLink = document.createElement('a');
                    artistLink.href = '#';
                    artistLink.textContent = artist.name;
                    artistLink.style.color = '#007bff';
                    artistLink.style.marginRight = '5px';
                    artistLink.onclick = (event) => {
                        event.stopPropagation();
                        window.parent.postMessage({ action: 'navigateTo', url: `artist.html?uuid=${artist.uuid}` }, '*');
                    };
                    songArtist.appendChild(artistLink);
                });

                const albumName = document.createElement('a');
                albumName.className = 'album-name';
                albumName.textContent = song.album.name;
                albumName.href = `album.html?uuid=${song.album.uuid}`;
                albumName.onclick = (event) => {
                    event.stopPropagation();
                    window.parent.postMessage({ action: 'navigateTo', url: `album.html?uuid=${song.album.uuid}` }, '*'); // 跳转到专辑详情页
                };

                songDetails.appendChild(songTitle);
                songDetails.appendChild(songArtist);
                songDetails.appendChild(albumName);
                songInfo.appendChild(songNumber);
                songInfo.appendChild(songArt);
                songInfo.appendChild(songDetails);

                const songActions = document.createElement('div');
                songActions.className = 'actions';

                const likedIcon = document.createElement('span');
                likedIcon.className = song.is_liked ? 'text-danger liked' : 'liked';
                likedIcon.innerHTML = song.is_liked ? '&#9829;' : '&#9825;';
                likedIcon.addEventListener('click', (event) => {
                    event.stopPropagation();
                    toggleSongLike(song.uuid);
                });
                songActions.appendChild(likedIcon);

                songItem.appendChild(songInfo);
                songItem.appendChild(songActions);
                songList.appendChild(songItem);
            });
        }

        async function fetchAndPlaySong(index) {
            currentIndex = index;
            const uuid = songs[index].uuid;
            try {
                console.log('Fetching song details for uuid:', uuid);
                const response = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                if (response.ok) {
                    const song = await response.json();
                    console.log('Fetched song details:', song);
                    playSong(song);
                } else {
                    console.error('Failed to fetch song details:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function playSong(song) {
            try {
                console.log('Playing song:', song);
                if (!song.file_path) {
                    console.error('Error: song.file_path is undefined');
                    return;
                }

                const songArtBlob = await fetchFile(song.song_art_path);
                let songArtUrl = 'default_cover.png';
                if (songArtBlob) {
                    songArtUrl = URL.createObjectURL(songArtBlob);
                }

                window.parent.postMessage({
                    action: 'playSong',
                    song: {
                        filePath: song.file_path,
                        art: songArtUrl,
                        title: song.name,
                        artists: song.artists.map(artist => ({ name: artist.name, uuid: artist.uuid })),
                        album: song.album.name,
                        albumUUID: song.album.uuid,
                        isLiked: song.is_liked,
                        uuid: song.uuid
                    }
                }, '*');

                currentSong = song;
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function toggleSongLike(uuid) {
            try {
                const response = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                if (response.ok) {
                    const song = await response.json();
                    let likeResponse;
                    if (song.is_liked) {
                        likeResponse = await fetch(`${apiBaseUrl}/unlike_song/${uuid}`);
                    } else {
                        likeResponse = await fetch(`${apiBaseUrl}/like_song/${uuid}`);
                    }

                    if (likeResponse.ok) {
                        const updatedSongResponse = await fetch(`${apiBaseUrl}/show_song/${uuid}`);
                        if (updatedSongResponse.ok) {
                            const updatedSong = await updatedSongResponse.json();
                            window.parent.postMessage({
                                action: 'updateSongLike',
                                uuid: updatedSong.uuid,
                                isLiked: updatedSong.is_liked
                            }, '*');

                            updateSongListIcon(updatedSong.uuid, updatedSong.is_liked);
                        }
                    } else {
                        console.error('Failed to toggle like status:', likeResponse.statusText);
                    }
                } else {
                    console.error('Failed to fetch song details:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        async function toggleArtistLike() {
            try {
                const response = await fetch(`${apiBaseUrl}/show_artist/${artistUUID}`);
                if (response.ok) {
                    const artist = await response.json();
                    let likeResponse;
                    if (artist.is_liked) {
                        likeResponse = await fetch(`${apiBaseUrl}/unlike_artist/${artistUUID}`);
                    } else {
                        likeResponse = await fetch(`${apiBaseUrl}/like_artist/${artistUUID}`);
                    }

                    if (likeResponse.ok) {
                        const updatedArtistResponse = await fetch(`${apiBaseUrl}/show_artist/${artistUUID}`);
                        if (updatedArtistResponse.ok) {
                            const updatedArtist = await updatedArtistResponse.json();
                            const artistLike = document.getElementById('artist-like');
                            artistLike.className = updatedArtist.is_liked ? 'artist-like text-danger' : 'artist-like';
                            artistLike.innerHTML = updatedArtist.is_liked ? '&#9829;' : '&#9825;';
                        }
                    } else {
                        console.error('Failed to toggle like status:', likeResponse.statusText);
                    }
                } else {
                    console.error('Failed to fetch artist details:', response.statusText);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        window.addEventListener('message', (event) => {
            if (event.data.action === 'previous') {
                playPreviousSong();
            } else if (event.data.action === 'next') {
                playNextSong();
            } else if (event.data.action === 'toggleLike') {
                toggleLike();
            } else if (event.data.action === 'updateSongLike') {
                updateSongListIcon(event.data.uuid, event.data.isLiked);
            }
        });

        function updateSongListIcon(uuid, isLiked) {
            const songItem = document.getElementById(`song-${uuid}`);
            if (songItem) {
                const likedIcon = songItem.querySelector('.actions span');
                likedIcon.className = isLiked ? 'text-danger liked' : 'liked';
                likedIcon.innerHTML = isLiked ? '&#9829;' : '&#9825;';
            }
        }

        let isDebounced = false;

        function playNextSong() {
            if (isDebounced) return;

            isDebounced = true;
            setTimeout(() => { isDebounced = false; }, 1000);

            console.log("next song!");
            currentIndex = (currentIndex + 1) % songs.length;
            fetchAndPlaySong(currentIndex);
        }

        function playPreviousSong() {
            if (isDebounced) return;

            isDebounced = true;
            setTimeout(() => { isDebounced = false; }, 1000);

            currentIndex = (currentIndex - 1 + songs.length) % songs.length;
            fetchAndPlaySong(currentIndex);
        }

        async function toggleLike() {
            if (!currentSong) {
                console.error('Error: No song is currently being played.');
                return;
            }
            await toggleSongLike(currentSong.uuid);
        }

        fetchArtistData();
    