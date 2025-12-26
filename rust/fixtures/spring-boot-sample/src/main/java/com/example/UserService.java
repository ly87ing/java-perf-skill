package com.example;

import org.springframework.stereotype.Service;
import org.springframework.scheduling.annotation.Async;
import java.util.List;
import java.util.stream.Collectors;

/**
 * 示例 Service - 包含典型性能问题
 */
@Service
public class UserService {

    private final UserRepository userRepository;

    public UserService(UserRepository userRepository) {
        this.userRepository = userRepository;
    }

    public List<User> findAllWithDetails() {
        List<User> users = userRepository.findAll();
        // N+1 问题：循环内查询
        for (User user : users) {
            user.setOrders(userRepository.findOrdersByUserId(user.getId()));
        }
        return users;
    }
    
    @Async  // P1: 未指定线程池
    public void processAsync() {
        // 使用默认 SimpleAsyncTaskExecutor
    }
    
    public User findById(Long id) {
        return userRepository.findById(id);
    }
    
    public void badMethod() {
        try {
            riskyOperation();
        } catch (Exception e) {
            // P0: 空 catch 块
        }
    }
    
    private void riskyOperation() throws Exception {
        // 风险操作
    }
}
